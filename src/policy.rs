use serde_json::{json, Value};

use crate::error::Error;
use crate::json_scan::{parse_json, reject_duplicate_keys};

/// Lifecycle policy text for one repository. It is not applied.
pub fn propose_policy(untagged_days: u32, keep: u32) -> Result<String, Error> {
    if untagged_days == 0 || keep == 0 {
        return Err(Error::new("policy counts must be positive"));
    }
    let value = json!({
        "rules": [
            {
                "rulePriority": 1,
                "description": format!("Expire untagged images older than {untagged_days} days"),
                "selection": {
                    "tagStatus": "untagged",
                    "countType": "sinceImagePushed",
                    "countUnit": "days",
                    "countNumber": untagged_days
                },
                "action": { "type": "expire" }
            },
            {
                "rulePriority": 2,
                "description": format!("Keep the newest {keep} tagged images"),
                "selection": {
                    "tagStatus": "tagged",
                    "tagPatternList": ["*"],
                    "countType": "imageCountMoreThan",
                    "countNumber": keep
                },
                "action": { "type": "expire" }
            }
        ]
    });
    let mut text = serde_json::to_string_pretty(&value).map_err(|_| Error::new("not JSON"))?;
    text.push('\n');
    validate_policy(&text)?;
    Ok(text)
}

/// Structural check against the lifecycle policy properties in the Amazon ECR User Guide.
pub fn validate_policy(text: &str) -> Result<(), Error> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    reject_duplicate_keys(text)?;
    let value: Value = parse_json(text)?;
    check_policy(&value)
}

fn check_policy(value: &Value) -> Result<(), Error> {
    let root = value
        .as_object()
        .ok_or_else(|| Error::new("lifecycle policy: rules must be an array"))?;
    if root.len() != 1 || !root.contains_key("rules") {
        return Err(Error::new("lifecycle policy: unexpected field"));
    }
    let rules = root
        .get("rules")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::new("lifecycle policy: rules must be an array"))?;
    if rules.is_empty() {
        return Err(Error::new("lifecycle policy: rules must be an array"));
    }
    let mut parsed = Vec::with_capacity(rules.len());
    for rule in rules {
        parsed.push(check_rule(rule)?);
    }
    let mut priorities = Vec::new();
    for rule in &parsed {
        if priorities.contains(&rule.priority) {
            return Err(Error::new("lifecycle policy: rulePriority must be unique"));
        }
        priorities.push(rule.priority);
    }
    let non_any_max = parsed
        .iter()
        .filter(|rule| !rule.tag_any)
        .map(|rule| rule.priority)
        .max();
    if let Some(max) = non_any_max {
        for rule in &parsed {
            if rule.tag_any && rule.priority <= max {
                return Err(Error::new(
                    "lifecycle policy: tagStatus any must have the highest rulePriority",
                ));
            }
        }
    }
    Ok(())
}

struct Checked {
    priority: u64,
    tag_any: bool,
}

fn check_rule(value: &Value) -> Result<Checked, Error> {
    let rule = value
        .as_object()
        .ok_or_else(|| Error::new("lifecycle policy: rule must be an object"))?;
    for key in rule.keys() {
        if !matches!(
            key.as_str(),
            "rulePriority" | "description" | "selection" | "action"
        ) {
            return Err(Error::new("lifecycle policy: unexpected field"));
        }
    }
    let priority = positive_int(
        rule.get("rulePriority")
            .ok_or_else(|| Error::new("lifecycle policy: rulePriority is required"))?,
    )?;
    if let Some(description) = rule.get("description") {
        if !description.is_string() {
            return Err(Error::new("lifecycle policy: description must be a string"));
        }
    }
    let selection = rule
        .get("selection")
        .and_then(Value::as_object)
        .ok_or_else(|| Error::new("lifecycle policy: selection is required"))?;
    let tag_any = check_selection(selection)?;
    let action = rule
        .get("action")
        .and_then(Value::as_object)
        .ok_or_else(|| Error::new("lifecycle policy: action is required"))?;
    check_action(action, selection)?;
    Ok(Checked { priority, tag_any })
}

fn check_selection(selection: &serde_json::Map<String, Value>) -> Result<bool, Error> {
    for key in selection.keys() {
        if !matches!(
            key.as_str(),
            "tagStatus"
                | "tagPatternList"
                | "tagPrefixList"
                | "storageClass"
                | "countType"
                | "countUnit"
                | "countNumber"
        ) {
            return Err(Error::new("lifecycle policy: unexpected field"));
        }
    }
    let tag_status = selection
        .get("tagStatus")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::new("lifecycle policy: tagStatus is required"))?;
    if !matches!(tag_status, "tagged" | "untagged" | "any") {
        return Err(Error::new(
            "lifecycle policy: tagStatus is not a supported value",
        ));
    }
    let patterns = optional_strings(selection, "tagPatternList")?;
    let prefixes = optional_strings(selection, "tagPrefixList")?;
    match tag_status {
        "tagged" => {
            if patterns.is_some() == prefixes.is_some() {
                return Err(Error::new(
                    "lifecycle policy: tagged rules need tagPatternList or tagPrefixList, not both",
                ));
            }
        }
        "untagged" | "any" if patterns.is_some() || prefixes.is_some() => {
            return Err(Error::new(
                "lifecycle policy: untagged and any rules omit tag lists",
            ));
        }
        "untagged" | "any" => {}
        _ => {}
    }
    if let Some(list) = &patterns {
        for pattern in list {
            if pattern.bytes().filter(|byte| *byte == b'*').count() > 4 {
                return Err(Error::new(
                    "lifecycle policy: a tag pattern has more than four wildcards",
                ));
            }
        }
    }
    let count_type = selection
        .get("countType")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::new("lifecycle policy: countType is required"))?;
    if !matches!(
        count_type,
        "imageCountMoreThan" | "sinceImagePushed" | "sinceImagePulled" | "sinceImageTransitioned"
    ) {
        return Err(Error::new(
            "lifecycle policy: countType is not a supported value",
        ));
    }
    let timed = matches!(
        count_type,
        "sinceImagePushed" | "sinceImagePulled" | "sinceImageTransitioned"
    );
    match selection.get("countUnit") {
        Some(unit) if timed => {
            if unit.as_str() != Some("days") {
                return Err(Error::new("lifecycle policy: countUnit must be days"));
            }
        }
        Some(_) => {
            return Err(Error::new(
                "lifecycle policy: countUnit is only used with a sinceImage count type",
            ));
        }
        None if timed => {
            return Err(Error::new("lifecycle policy: countUnit is required"));
        }
        None => {}
    }
    let count_number = positive_int(
        selection
            .get("countNumber")
            .ok_or_else(|| Error::new("lifecycle policy: countNumber is required"))?,
    )?;
    let storage = selection.get("storageClass").and_then(Value::as_str);
    match storage {
        None => {}
        Some("standard" | "archive") => {}
        Some(_) => {
            return Err(Error::new(
                "lifecycle policy: storageClass is not a supported value",
            ));
        }
    }
    let class = storage.unwrap_or("standard");
    if count_type == "sinceImageTransitioned" {
        if class != "archive" {
            return Err(Error::new(
                "lifecycle policy: sinceImageTransitioned requires storageClass archive",
            ));
        }
        if count_number < 90 {
            return Err(Error::new(
                "lifecycle policy: sinceImageTransitioned countNumber must be at least 90",
            ));
        }
    } else if class != "standard" {
        return Err(Error::new(
            "lifecycle policy: this countType only supports storageClass standard",
        ));
    }
    Ok(tag_status == "any")
}

fn check_action(
    action: &serde_json::Map<String, Value>,
    selection: &serde_json::Map<String, Value>,
) -> Result<(), Error> {
    for key in action.keys() {
        if !matches!(key.as_str(), "type" | "targetStorageClass") {
            return Err(Error::new("lifecycle policy: unexpected field"));
        }
    }
    let kind = action
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::new("lifecycle policy: action type is required"))?;
    let count_type = selection
        .get("countType")
        .and_then(Value::as_str)
        .unwrap_or("");
    match kind {
        "expire" => {
            if action.contains_key("targetStorageClass") {
                return Err(Error::new(
                    "lifecycle policy: expire does not take targetStorageClass",
                ));
            }
            if count_type == "sinceImagePulled" {
                return Err(Error::new(
                    "lifecycle policy: sinceImagePulled requires action transition",
                ));
            }
        }
        "transition" => {
            if action.get("targetStorageClass").and_then(Value::as_str) != Some("archive") {
                return Err(Error::new(
                    "lifecycle policy: transition requires targetStorageClass archive",
                ));
            }
            if count_type == "sinceImageTransitioned" {
                return Err(Error::new(
                    "lifecycle policy: sinceImageTransitioned requires action expire",
                ));
            }
        }
        _ => {
            return Err(Error::new(
                "lifecycle policy: action type is not a supported value",
            ));
        }
    }
    Ok(())
}

fn optional_strings(
    obj: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<Vec<String>>, Error> {
    let Some(value) = obj.get(key) else {
        return Ok(None);
    };
    let list = value
        .as_array()
        .ok_or_else(|| Error::new("lifecycle policy: tag list must be an array"))?;
    if list.is_empty() {
        return Err(Error::new("lifecycle policy: tag list must not be empty"));
    }
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        let text = item
            .as_str()
            .ok_or_else(|| Error::new("lifecycle policy: tag list must be strings"))?;
        if text.is_empty() {
            return Err(Error::new("lifecycle policy: tag list must be strings"));
        }
        out.push(text.to_string());
    }
    Ok(Some(out))
}

fn positive_int(value: &Value) -> Result<u64, Error> {
    let Some(number) = value.as_number() else {
        return Err(Error::new(
            "lifecycle policy: count must be a positive integer",
        ));
    };
    let text = number.to_string();
    if text.bytes().any(|byte| !byte.is_ascii_digit()) || text == "0" {
        return Err(Error::new(
            "lifecycle policy: count must be a positive integer",
        ));
    }
    text.parse()
        .map_err(|_| Error::new("lifecycle policy: count must be a positive integer"))
}

#[cfg(test)]
mod tests {
    use super::{propose_policy, validate_policy};

    fn ok(text: &str) {
        validate_policy(text).unwrap();
    }

    fn bad(text: &str, needle: &str) {
        let err = validate_policy(text).unwrap_err();
        assert!(
            err.message().contains(needle),
            "{} missing {needle}",
            err.message()
        );
    }

    #[test]
    fn claim_c10_proposal_is_two_expire_rules() {
        let text = propose_policy(14, 10).unwrap();
        assert!(text.contains("\"rulePriority\": 1"));
        assert!(text.contains("\"tagStatus\": \"untagged\""));
        assert!(text.contains("\"countNumber\": 14"));
        assert!(text.contains("\"countNumber\": 10"));
        assert!(text.contains("\"tagPatternList\": ["));
        assert!(text.contains("\"*\""));
        assert!(!text.contains("not_executed"));
        assert!(!text.contains(concat!("Put", "LifecyclePolicy")));
        validate_policy(&text).unwrap();
        assert!(propose_policy(0, 10).is_err());
    }

    #[test]
    fn claim_c11_documented_examples_are_valid() {
        ok(
            r#"{"rules":[{"rulePriority":1,"description":"Expire images older than 14 days","selection":{"tagStatus":"tagged","tagPatternList":["prod*"],"countType":"sinceImagePushed","countUnit":"days","countNumber":14},"action":{"type":"expire"}}]}"#,
        );
        ok(
            r#"{"rules":[{"rulePriority":1,"description":"Archive images not pulled in 90 days","selection":{"tagStatus":"any","countType":"sinceImagePulled","countUnit":"days","countNumber":90},"action":{"type":"transition","targetStorageClass":"archive"}}]}"#,
        );
        ok(
            r#"{"rules":[{"rulePriority":1,"description":"Expire images archived for more than 365 days","selection":{"tagStatus":"any","storageClass":"archive","countType":"sinceImageTransitioned","countUnit":"days","countNumber":365},"action":{"type":"expire"}}]}"#,
        );
        ok(
            r#"{"rules":[{"rulePriority":1,"description":"Keep only one untagged image, expire all others","selection":{"tagStatus":"untagged","countType":"imageCountMoreThan","countNumber":1},"action":{"type":"expire"}}]}"#,
        );
        ok(
            r#"{"rules":[{"rulePriority":1,"description":"Rule 1","selection":{"tagStatus":"tagged","tagPatternList":["alpha*"],"countType":"imageCountMoreThan","countNumber":1},"action":{"type":"expire"}},{"rulePriority":2,"description":"Rule 2","selection":{"tagStatus":"untagged","countType":"sinceImagePushed","countUnit":"days","countNumber":1},"action":{"type":"expire"}},{"rulePriority":3,"description":"Rule 3","selection":{"tagStatus":"any","countType":"imageCountMoreThan","countNumber":1},"action":{"type":"expire"}}]}"#,
        );
        ok(
            r#"{"rules":[{"rulePriority":1,"description":"Archive images older than 30 days","selection":{"tagStatus":"any","countType":"sinceImagePushed","countUnit":"days","countNumber":30},"action":{"type":"transition","targetStorageClass":"archive"}},{"rulePriority":2,"description":"Expire images archived for more than 365 days","selection":{"tagStatus":"any","storageClass":"archive","countType":"sinceImageTransitioned","countUnit":"days","countNumber":365},"action":{"type":"expire"}}]}"#,
        );
        ok(
            r#"{"rules":[{"rulePriority":1,"description":"patterns","selection":{"tagStatus":"tagged","tagPatternList":["*test*1*2*3","test*1*2*3*"],"countType":"imageCountMoreThan","countNumber":1},"action":{"type":"expire"}}]}"#,
        );
    }

    #[test]
    fn claim_c12_tag_lists_and_wildcards() {
        bad(
            r#"{"rules":[{"rulePriority":1,"selection":{"tagStatus":"tagged","tagPatternList":["a"],"tagPrefixList":["a"],"countType":"imageCountMoreThan","countNumber":1},"action":{"type":"expire"}}]}"#,
            "not both",
        );
        bad(
            r#"{"rules":[{"rulePriority":1,"selection":{"tagStatus":"tagged","countType":"imageCountMoreThan","countNumber":1},"action":{"type":"expire"}}]}"#,
            "not both",
        );
        bad(
            r#"{"rules":[{"rulePriority":1,"selection":{"tagStatus":"untagged","tagPatternList":["a"],"countType":"sinceImagePushed","countUnit":"days","countNumber":1},"action":{"type":"expire"}}]}"#,
            "omit tag lists",
        );
        bad(
            r#"{"rules":[{"rulePriority":1,"selection":{"tagStatus":"tagged","tagPatternList":["test*1*2*3*4*5*6"],"countType":"imageCountMoreThan","countNumber":1},"action":{"type":"expire"}}]}"#,
            "four wildcards",
        );
    }

    #[test]
    fn claim_c13_pull_and_archive_actions() {
        bad(
            r#"{"rules":[{"rulePriority":1,"selection":{"tagStatus":"any","countType":"sinceImagePulled","countUnit":"days","countNumber":90},"action":{"type":"expire"}}]}"#,
            "sinceImagePulled requires action transition",
        );
        bad(
            r#"{"rules":[{"rulePriority":1,"selection":{"tagStatus":"any","storageClass":"archive","countType":"sinceImageTransitioned","countUnit":"days","countNumber":89},"action":{"type":"expire"}}]}"#,
            "at least 90",
        );
        bad(
            r#"{"rules":[{"rulePriority":1,"selection":{"tagStatus":"any","countType":"sinceImageTransitioned","countUnit":"days","countNumber":90},"action":{"type":"expire"}}]}"#,
            "requires storageClass archive",
        );
        ok(
            r#"{"rules":[{"rulePriority":1,"selection":{"tagStatus":"any","storageClass":"archive","countType":"sinceImageTransitioned","countUnit":"days","countNumber":90},"action":{"type":"expire"}}]}"#,
        );
    }

    #[test]
    fn claim_c14_priority_count_and_units() {
        bad(
            r#"{"rules":[{"rulePriority":2,"selection":{"tagStatus":"tagged","tagPrefixList":["prod"],"countType":"imageCountMoreThan","countNumber":1},"action":{"type":"expire"}},{"rulePriority":1,"selection":{"tagStatus":"any","countType":"imageCountMoreThan","countNumber":1},"action":{"type":"expire"}}]}"#,
            "highest rulePriority",
        );
        bad(
            r#"{"rules":[{"rulePriority":1,"selection":{"tagStatus":"untagged","countType":"imageCountMoreThan","countNumber":0},"action":{"type":"expire"}}]}"#,
            "positive integer",
        );
        bad(
            r#"{"rules":[{"rulePriority":1,"selection":{"tagStatus":"untagged","countType":"imageCountMoreThan","countUnit":"days","countNumber":1},"action":{"type":"expire"}}]}"#,
            "countUnit is only used",
        );
        bad(
            r#"{"rules":[{"rulePriority":1,"selection":{"tagStatus":"untagged","countType":"sinceImagePushed","countNumber":1},"action":{"type":"expire"}}]}"#,
            "countUnit is required",
        );
        bad(
            r#"{"rules":[{"rulePriority":1,"selection":{"tagStatus":"untagged","countType":"sinceImagePushed","countUnit":"days","countNumber":1},"action":{"type":"expire","targetStorageClass":"archive"}}]}"#,
            "expire does not take",
        );
        bad(
            r#"{"rules":[{"rulePriority":1,"selection":{"tagStatus":"any","countType":"sinceImagePushed","countUnit":"days","countNumber":1},"action":{"type":"transition"}}]}"#,
            "targetStorageClass archive",
        );
        bad(
            r#"{"rules":[{"rulePriority":1,"selection":{"tagStatus":"untagged","countType":"sinceImagePushed","countUnit":"weeks","countNumber":1},"action":{"type":"expire"}}]}"#,
            "countUnit must be days",
        );
        bad(
            r#"{"rules":[{"rulePriority":1,"rulePriority":2,"selection":{"tagStatus":"untagged","countType":"imageCountMoreThan","countNumber":1},"action":{"type":"expire"}}]}"#,
            "duplicate key",
        );
    }

    #[test]
    fn rule_priorities_must_be_unique() {
        bad(
            r#"{"rules":[{"rulePriority":1,"selection":{"tagStatus":"tagged","tagPrefixList":["prod"],"countType":"imageCountMoreThan","countNumber":1},"action":{"type":"expire"}},{"rulePriority":1,"selection":{"tagStatus":"untagged","countType":"imageCountMoreThan","countNumber":1},"action":{"type":"expire"}}]}"#,
            "rulePriority must be unique",
        );
    }

    #[test]
    fn unknown_fields_are_rejected_at_every_level() {
        let rule = |extra_root: &str, extra_rule: &str, extra_sel: &str, extra_act: &str| {
            format!(
                r#"{{{extra_root}"rules":[{{{extra_rule}"rulePriority":1,"selection":{{{extra_sel}"tagStatus":"untagged","countType":"imageCountMoreThan","countNumber":1}},"action":{{{extra_act}"type":"expire"}}}}]}}"#
            )
        };
        ok(&rule("", "", "", ""));
        bad(&rule(r#""extra":1,"#, "", "", ""), "unexpected field");
        bad(&rule("", r#""extra":1,"#, "", ""), "unexpected field");
        bad(&rule("", "", r#""extra":1,"#, ""), "unexpected field");
        bad(&rule("", "", "", r#""extra":1,"#), "unexpected field");
    }

    #[test]
    fn archive_storage_class_needs_sinceimagetransitioned() {
        bad(
            r#"{"rules":[{"rulePriority":1,"selection":{"tagStatus":"any","storageClass":"archive","countType":"sinceImagePushed","countUnit":"days","countNumber":30},"action":{"type":"expire"}}]}"#,
            "only supports storageClass standard",
        );
    }

    #[test]
    fn proposal_counts_must_be_positive() {
        for (days, keep) in [(0, 10), (14, 0), (0, 0)] {
            assert_eq!(
                propose_policy(days, keep).unwrap_err().message(),
                "policy counts must be positive"
            );
        }
    }

    #[test]
    fn rule_and_root_shape_errors() {
        bad("[]", "rules must be an array");
        bad(r#"{"rules":{}}"#, "rules must be an array");
        bad(r#"{"rules":[]}"#, "rules must be an array");
        bad(r#"{"rules":[1]}"#, "rule must be an object");
    }

    #[test]
    fn rule_level_required_fields() {
        let sel = r#""selection":{"tagStatus":"untagged","countType":"imageCountMoreThan","countNumber":1}"#;
        let act = r#""action":{"type":"expire"}"#;
        bad(
            &format!(r#"{{"rules":[{{{sel},{act}}}]}}"#),
            "rulePriority is required",
        );
        bad(
            &format!(r#"{{"rules":[{{"rulePriority":1,"description":5,{sel},{act}}}]}}"#),
            "description must be a string",
        );
        bad(
            &format!(r#"{{"rules":[{{"rulePriority":1,{act}}}]}}"#),
            "selection is required",
        );
        bad(
            &format!(r#"{{"rules":[{{"rulePriority":1,{sel}}}]}}"#),
            "action is required",
        );
    }

    fn with_selection(selection: &str) -> String {
        format!(
            r#"{{"rules":[{{"rulePriority":1,"selection":{{{selection}}},"action":{{"type":"expire"}}}}]}}"#
        )
    }

    #[test]
    fn selection_required_and_unsupported_values() {
        bad(
            &with_selection(r#""countType":"imageCountMoreThan","countNumber":1"#),
            "tagStatus is required",
        );
        bad(
            &with_selection(
                r#""tagStatus":"other","countType":"imageCountMoreThan","countNumber":1"#,
            ),
            "tagStatus is not a supported value",
        );
        bad(
            &with_selection(r#""tagStatus":"untagged","countNumber":1"#),
            "countType is required",
        );
        bad(
            &with_selection(r#""tagStatus":"untagged","countType":"other","countNumber":1"#),
            "countType is not a supported value",
        );
        bad(
            &with_selection(r#""tagStatus":"untagged","countType":"imageCountMoreThan""#),
            "countNumber is required",
        );
        bad(
            &with_selection(
                r#""tagStatus":"untagged","storageClass":"cold","countType":"imageCountMoreThan","countNumber":1"#,
            ),
            "storageClass is not a supported value",
        );
    }

    #[test]
    fn tag_list_shape_errors() {
        for (list, needle) in [
            (r#""tagPatternList":"a""#, "tag list must be an array"),
            (r#""tagPatternList":[]"#, "tag list must not be empty"),
            (r#""tagPatternList":[1]"#, "tag list must be strings"),
            (r#""tagPrefixList":[""]"#, "tag list must be strings"),
        ] {
            bad(
                &with_selection(&format!(
                    r#""tagStatus":"tagged",{list},"countType":"imageCountMoreThan","countNumber":1"#
                )),
                needle,
            );
        }
    }

    #[test]
    fn action_errors() {
        let rule = |action: &str, count_type: &str, extra: &str| {
            format!(
                r#"{{"rules":[{{"rulePriority":1,"selection":{{"tagStatus":"any",{extra}"countType":"{count_type}","countUnit":"days","countNumber":90}},"action":{action}}}]}}"#
            )
        };
        bad(
            &rule("{}", "sinceImagePushed", ""),
            "action type is required",
        );
        bad(
            &rule(r#"{"type":"delete"}"#, "sinceImagePushed", ""),
            "action type is not a supported value",
        );
        bad(
            &rule(
                r#"{"type":"transition","targetStorageClass":"archive"}"#,
                "sinceImageTransitioned",
                r#""storageClass":"archive","#,
            ),
            "sinceImageTransitioned requires action expire",
        );
    }
}
