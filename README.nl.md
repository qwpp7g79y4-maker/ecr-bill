# ecr-bill

[![crates.io](https://img.shields.io/crates/v/ecr-bill.svg)](https://crates.io/crates/ecr-bill) [![downloads](https://img.shields.io/crates/d/ecr-bill.svg)](https://crates.io/crates/ecr-bill) [![CI](https://github.com/qwpp7g79y4-maker/ecr-bill/actions/workflows/ci.yml/badge.svg)](https://github.com/qwpp7g79y4-maker/ecr-bill/actions/workflows/ci.yml) [![PEPAI checked](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Fqwpp7g79y4-maker%2Fecr-bill%2Fmain%2F.github%2Fbadge.json)](QUALITY.md) [![MSRV 1.85](https://img.shields.io/badge/MSRV-1.85-blue.svg)](https://github.com/qwpp7g79y4-maker/ecr-bill/blob/main/Cargo.toml) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/qwpp7g79y4-maker/ecr-bill#license)

Schat de opslagkosten van Amazon ECR uit `describe-images`-JSON. Leest alleen bestanden, geen netwerk.

## Waarom

Een `aws ecr describe-images`-export noemt per image de grootte, de tags en de status. ecr-bill prijst die JSON op je eigen machine, per repository, toont untagged en oude images en drukt een lifecycle-policy als JSON af. Het roept AWS niet aan en heeft geen account nodig.

## Installatie

Versie 1.0.2. Van crates.io:

```sh
cargo install ecr-bill
```

Of bouw uit de broncode.

Met Rust 1.85 of nieuwer, vanuit een clone van deze repository:

```sh
cargo install --locked --path .
ecr-bill --help
```

De tests die `cargo test --locked` draait staan per testbestand in [QUALITY.md](QUALITY.md).

## Snelstart

`examples/describe-images.json` staat in de repository en bevat twee Standard-images in repository `app`, elk 1 GiB. Eén is getagd `v1` en is gepusht op 2024-01-15. De andere heeft geen tags en is gepusht op 2020-01-01. Het account-id in het bestand wordt niet getoond. Hieronder schrijft niets een bestand: draai de commando's vanuit de root van een clone, met `ecr-bill` op je `PATH`.

Een test draait elk console-blok hieronder en vergelijkt het met deze binary. `--as-of` is middernacht UTC. De tarieven zijn US East (N. Virginia) van <https://aws.amazon.com/ecr/pricing/>, gelezen op 2026-11-09.

```console
$ ecr-bill --as-of 2026-11-09 examples/describe-images.json
ecr-bill
Estimate from list prices, not your AWS invoice (free tier, credits, discounts not included)
source: https://aws.amazon.com/ecr/pricing/
as of: 2026-11-09
region: us-east-1 (US East (N. Virginia))
free tier not applied
not applied: retrieval 0.03 USD per GB; archive minimum 90 days

repositories
app images=2 bytes=2147483648 untagged=1 old=2 estimated_usd_month=0.20
total images=2 bytes=2147483648 untagged=1 estimated_usd_month=0.20

untagged
app sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb bytes=1073741824 tags=0

old
as-of: 2026-11-09
older_than_days: 30
app sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa age_days=1029 tags=1 bytes=1073741824
app sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb age_days=2504 tags=0 bytes=1073741824

lifecycle
Not executed
{
  "rules": [
    {
      "rulePriority": 1,
      "description": "Expire untagged images older than 14 days",
      "selection": {
        "tagStatus": "untagged",
        "countType": "sinceImagePushed",
        "countUnit": "days",
        "countNumber": 14
      },
      "action": {
        "type": "expire"
      }
    },
    {
      "rulePriority": 2,
      "description": "Keep the newest 10 tagged images",
      "selection": {
        "tagStatus": "tagged",
        "tagPatternList": [
          "*"
        ],
        "countType": "imageCountMoreThan",
        "countNumber": 10
      },
      "action": {
        "type": "expire"
      }
    }
  ]
}
```

## Aan de slag in 5 minuten

Vijf commando's, offline, op het bestand in `examples/`. Ze hebben geen account en geen netwerk nodig en schrijven niets. Draai ze vanuit de root van een clone, met `ecr-bill` op je `PATH` (na `cargo install --locked --path .`, of gebruik `target/release/ecr-bill`). Een test draait de commando's in dit blok letterlijk.

```sh
ecr-bill --as-of 2026-11-09 examples/describe-images.json
ecr-bill --json --as-of 2026-11-09 examples/describe-images.json
ecr-bill --as-of 2026-11-09 --older-than-days 2000 examples/describe-images.json
ecr-bill --as-of 2026-11-09 --keep 1 --untagged-days 7 examples/describe-images.json
ecr-bill --prices prices/ecr-storage.json examples/describe-images.json
```

1. Het tekstrapport, met de lijst van oude images gemeten tot 2026-11-09.
2. Hetzelfde als één JSON-object.
3. Alleen images die meer dan 2000 dagen vóór die datum zijn gepusht tellen als oud.
4. Een andere lifecycle-policy: bewaar 1 getagd image, laat untagged images na 7 dagen verlopen.
5. Het ingebouwde prijsbestand expliciet genoemd; zonder `--as-of`, dus de lijst van oude images blijft weg.

Alle vijf eindigen met 0. Voor je eigen account: bewaar `aws ecr describe-images --repository-name NAAM` in een bestand en geef het mee als `FILE`.

## Opties

Elke optie uit `--help`.

- `FILE...`: `aws ecr describe-images`-JSON-bestanden (minstens één).
- `--prices <FILE>`: prijsbestand; standaard de ingebouwde US East (N. Virginia)-lijst.
- `--as-of <YYYY-MM-DD>`: datum waartoe leeftijden worden gemeten, middernacht UTC. Zonder deze optie blijft de lijst van oude images weg.
- `--older-than-days <N>`: een image is oud als hij meer dan N dagen vóór `--as-of` is gepusht (standaard 30, 1 tot 36500).
- `--untagged-days <N>`: untagged images ouder dan N dagen staan in de voorgestelde expire-regel (standaard 14, 1 tot 36500).
- `--keep <N>`: getagde images boven N staan in de voorgestelde expire-regel (standaard 10, 1 tot 10000).
- `--output <FILE>`: schrijf het rapport naar dit bestand in plaats van naar standaarduitvoer (standaarduitvoer blijft leeg). Het rapport gaat naar een nieuw bestand in dezelfde map en wordt over `FILE` heen hernoemd, dus `FILE` bevat de oude inhoud of het hele nieuwe rapport. Een bestaand gewoon bestand wordt vervangen en houdt zijn rechten (`chmod 600` blijft 600); een nieuw bestand krijgt de standaardmodus. Het tijdelijke bestand heeft een eigen korte naam (`.ecr-bill-<pid>.tmp`), dus een bestandsnaam tegen de grens van 255 bytes werkt. Een symbolische link en alles wat geen gewoon bestand is worden geweigerd, en een ontbrekende map wordt niet aangemaakt: afsluitcode 2 met `ecr-bill: could not write FILE: reason`. Een mislukte of onderbroken run laat een bestaand `FILE` zoals het was.
- `--json`: druk JSON af in plaats van de teksttabel. Het object begint met `"schema_version": 1`, de versie van de opbouw van deze JSON; die verandert alleen als een veld verdwijnt of zijn betekenis verandert. Een mislukte run drukt geen JSON af, alleen één foutregel op standaardfout.
- `--md`: druk Markdown-tabellen af in plaats van de teksttabel. De bedragen zijn dezelfde strings als in het tekst- en JSON-rapport. Een pipe, een backslash en de andere Markdown-tekens in een waarde krijgen een backslash; stuur- en regeleindetekens worden eerst als escape geschreven. Niet samen met `--json` (gebruiksfout, afsluitcode 2).
- `--quiet`: druk alleen het rapport en fouten af. De waarschuwingsregels op standaardfout (de waarschuwing over de prijsleeftijd en, met `--json`, de image-waarschuwingen) worden niet geschreven; ze blijven in het rapport. Standaarduitvoer en afsluitcode veranderen niet. Samen met `--verbose` is het een gebruiksfout (afsluitcode 2).
- `--verbose`: voeg voortgangsregels toe op standaardfout (`ecr-bill: reading N files`, `ecr-bill: reading FILE` met stuurtekens geëscaped, `ecr-bill: done`). Standaarduitvoer en afsluitcode veranderen niet.
- `-h`, `--help`: toon hulp. `-V`, `--version`: toon de versie.

## Voorbeeld

Hetzelfde bestand als JSON. Een test vergelijkt dit blok met de binary.

```console
$ ecr-bill --json --as-of 2026-11-09 examples/describe-images.json
{
  "schema_version": 1,
  "estimate": true,
  "source_url": "https://aws.amazon.com/ecr/pricing/",
  "source_date": "2026-11-09",
  "region": "us-east-1",
  "region_name": "US East (N. Virginia)",
  "free_tier_applied": false,
  "retrieval_usd_per_gb": "0.03",
  "archive_min_days": 90,
  "retrieval_applied": false,
  "as_of": "2026-11-09",
  "older_than_days": 30,
  "untagged_days": 14,
  "keep": 10,
  "images": 2,
  "bytes": "2147483648",
  "untagged_images": 1,
  "estimated_usd_month": "0.20",
  "repositories": [
    {
      "repository": "app",
      "images": 2,
      "bytes": "2147483648",
      "untagged": 1,
      "old": 2,
      "estimated_usd_month": "0.20"
    }
  ],
  "untagged": [
    {
      "repository": "app",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "bytes": "1073741824",
      "tags": 0
    }
  ],
  "old": [
    {
      "repository": "app",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "bytes": "1073741824",
      "tags": 1,
      "age_days": 1029
    },
    {
      "repository": "app",
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "bytes": "1073741824",
      "tags": 0,
      "age_days": 2504
    }
  ],
  "lifecycle": {
    "not_executed": true,
    "policy": {
      "rules": [
        {
          "rulePriority": 1,
          "description": "Expire untagged images older than 14 days",
          "selection": {
            "tagStatus": "untagged",
            "countType": "sinceImagePushed",
            "countUnit": "days",
            "countNumber": 14
          },
          "action": {
            "type": "expire"
          }
        },
        {
          "rulePriority": 2,
          "description": "Keep the newest 10 tagged images",
          "selection": {
            "tagStatus": "tagged",
            "tagPatternList": [
              "*"
            ],
            "countType": "imageCountMoreThan",
            "countNumber": 10
          },
          "action": {
            "type": "expire"
          }
        }
      ]
    }
  },
  "warnings": []
}
```

Hetzelfde rapport als Markdown. Een test vergelijkt dit blok met de binary.

```sh
ecr-bill --md --as-of 2026-11-09 examples/describe-images.json
```

````markdown
# ecr-bill

Estimate from list prices, not your AWS invoice (free tier, credits, discounts not included)

Source: https://aws.amazon.com/ecr/pricing/, price date 2026-11-09. Region: us-east-1 (US East (N. Virginia)). Free tier not applied. Not applied: retrieval 0.03 USD per GB; archive minimum 90 days.

## Repositories

| Repository | Images | Bytes | Untagged | Old | Estimated USD per month |
| --- | ---: | ---: | ---: | ---: | ---: |
| app | 2 | 2147483648 | 1 | 2 | 0.20 |
| **Total** | 2 | 2147483648 | 1 | | 0.20 |

## Untagged

| Repository | Digest | Bytes | Tags |
| --- | --- | ---: | ---: |
| app | sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb | 1073741824 | 0 |

## Old

As of 2026-11-09, older than 30 days.

| Repository | Digest | Age in days | Tags | Bytes |
| --- | --- | ---: | ---: | ---: |
| app | sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa | 1029 | 1 | 1073741824 |
| app | sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb | 2504 | 0 | 1073741824 |

## Lifecycle

Not executed.

```json
{
  "rules": [
    {
      "rulePriority": 1,
      "description": "Expire untagged images older than 14 days",
      "selection": {
        "tagStatus": "untagged",
        "countType": "sinceImagePushed",
        "countUnit": "days",
        "countNumber": 14
      },
      "action": {
        "type": "expire"
      }
    },
    {
      "rulePriority": 2,
      "description": "Keep the newest 10 tagged images",
      "selection": {
        "tagStatus": "tagged",
        "tagPatternList": [
          "*"
        ],
        "countType": "imageCountMoreThan",
        "countNumber": 10
      },
      "action": {
        "type": "expire"
      }
    }
  ]
}
```
````

## Hoe het telt

De ingebouwde tarieven zijn de US East (N. Virginia)-cijfers op <https://aws.amazon.com/ecr/pricing/> per 2026-11-09. ECR Standard, alle opslag, is 0,10 USD per GB-maand. ECR Archive is 0,10 USD per GB-maand voor de eerste 150 TB en 0,07 USD per GB-maand daarboven. Deze tool rekent 1 GB als 1.073.741.824 bytes en 1 TB als 1.024 GB, dus de archiefgrens is 150 maal 1.024 GB. `--prices` vervangt de hele prijsopgave.

C1. Eén `imageDetails`-element is één image. Dezelfde digest in twee bestanden telt twee keer.
C2. 1 GB is 1.073.741.824 bytes. 40 GB ECR Standard is 4,00 USD en 20 GB is 2,00 USD, zoals de voorbeelden op de prijspagina.
C3. Standard-opslag is 0,10 USD per GB-maand van https://aws.amazon.com/ecr/pricing/ per 2026-11-09, regio us-east-1.
C4. Een ontbrekende `imageStatus` is ECR Standard, de standaardklasse op de prijspagina.
C5. `ARCHIVED`-bytes kosten 0,10 USD per GB-maand voor de eerste 150 TB en 0,07 USD per GB-maand daarna. Ophalen (0,03 USD per GB) wordt niet opgeteld.
C6. `ACTIVATING` en elke andere status zijn onbeprijsd. Ze tellen nog wel mee als images en bytes. De waarschuwing noemt de status niet.
C7. Een image zonder `imageTags`, met null of met een lege lijst is untagged.
C8. Zonder `--as-of` wordt de opslag geprijsd en blijft de lijst van oude images weg. De klok is geen standaarddatum voor de rekening; hij wordt alleen gelezen voor de waarschuwing over de prijsleeftijd (zie Prijzen en verantwoordelijkheid).
C9. Een image is oud als zijn pushdatum meer dan `--older-than-days` vóór `--as-of` ligt. Een ontbrekende pushtijd blijft in de opslagrekening en wordt voor leeftijd overgeslagen. De waarschuwing noemt de digest niet.
C10. Het lifecycle-blok is JSON met een untagged `sinceImagePushed` expire-regel en een getagde `imageCountMoreThan` expire-regel. De woorden Not executed staan buiten die JSON.
C11. Die JSON, en de lifecycle-voorbeelden uit de Amazon ECR User Guide, slagen voor de schemacontrole in dit crate.
C12. `tagStatus` `tagged` heeft `tagPatternList` of `tagPrefixList` nodig, niet beide. Untagged en any laten beide weg. Een patroon mag hoogstens vier `*`-tekens hebben.
C13. `sinceImagePulled` is alleen geldig met actie `transition` naar archive. `sinceImageTransitioned` vereist `storageClass` `archive`, actie `expire` en een `countNumber` van minstens 90.
C14. Een regel met `tagStatus` `any` moet een hogere `rulePriority` hebben dan elke regel die niet `any` is. `countNumber` moet een positief geheel getal zijn. `countUnit` `days` hoort alleen bij de `sinceImage`-teltypes.
C15. Elk repositorybedrag wordt afgerond, half weg van nul, op de dichtstbijzijnde cent. Het dollartotaal is de som van de onafgeronde bedragen, daarna één keer op de cent afgerond. JSON-geldbedragen en bytetotalen zijn strings.
C16. Het registry-id wordt niet getoond.
C17. `--prices` vervangt de ingebouwde opgave. Een onbekend JSON-veld wordt genegeerd. Een dubbele sleutel is een fout met de melding `duplicate key`.
C18. Een ongeldige `imageSizeInBytes` wordt gemeld zonder de waarde of de digest.

Archiefdollars worden getrapt over de gearchiveerde bytes in de invoer. Ligt elke gearchiveerde byte op één tarief (de grens wordt niet overschreden, of beide tarieven zijn gelijk), dan wordt elke repository uit haar eigen gearchiveerde bytes geprijsd. Wordt de grens overschreden en verschillen de tarieven, dan krijgt elke repository haar evenredige deel van het getrapte totaal. Het archiefminimum van 90 dagen op de prijspagina wordt hier niet in rekening gebracht, omdat `describe-images` niet zegt hoe een verwijdering in dat venster zou vallen.

## Afsluitcodes

| Code | Betekenis |
| ---- | --------- |
| 0 | ok |
| 2 | een invoer- of gebruiksfout: onleesbaar of ongeldig bestand, een bestand boven 268435456 bytes, een waarde buiten het bereik, een onbekende optie, of standaarduitvoer of een `--output`-bestand dat niet geschreven kon worden |
| 130 | onderbroken door SIGINT (Ctrl-C), op Unix. Standaardfout is `ecr-bill: interrupted by SIGINT`, standaarduitvoer is leeg. Een signaal dat binnenkomt zodra het rapport klaar is en wordt geschreven, wordt genegeerd, zodat een rapport nooit half wordt geschreven |

## Beperkingen

Versie 1.0.2 prijst alleen Amazon ECR; andere registries vallen buiten het bereik (other registries). De lifecycle-regel wordt niet uitgevoerd (not executed): deze tool roept PutLifecyclePolicy niet aan. Er is geen AWS SDK en geen netwerkaanroep (no network). Een bestand boven 268435456 bytes wordt geweigerd. Een symlink naar een bestand wordt gevolgd. Een map of ander bestand dat geen gewoon bestand is, is een fout. Een pad dat tussen die controle en het openen door een fifo wordt vervangen, wordt net zo geweigerd, en het openen wacht niet op een schrijver. Het ingebouwde tarief is us-east-1. Ophalen (retrieval) zit niet in deze opslagrekening en het archiefminimum van 90 dagen wordt niet gerekend. De gratis laag van het account wordt niet toegepast (free tier is not applied): de private 500 MB en de publieke 50 GB worden niet afgetrokken. Dezelfde digest telt twee keer als twee bestanden hem noemen (same digest counts twice). Dataverkeer, managed signing en encryptie worden niet geprijsd. De leeftijdslijst en de policy zijn geen voorspelling (not a forecast). Een repositoryrij blijft op de cent. Het dollartotaal is de som van de onafgeronde (unrounded) bedragen, daarna één keer op de cent afgerond.

Invoerregels. Een bestand is UTF-8 JSON met één waarde; een BOM aan het begin wordt geaccepteerd, UTF-16 wordt geweigerd (`not UTF-8` met BOM, `not JSON` zonder), en een tweede waarde na de eerste is `not JSON`. Nesting is hoogstens 128 niveaus; dieper is de fout `JSON nesting is deeper than 128 levels`. Een dubbele sleutel in een object is een fout. `imageSizeInBytes` is een geheel getal van 0 tot 18446744073709551615, en een bytetotaal dat niet in 64 bits past is een fout. `imageStatus` wordt hoofdletterongevoelig gelezen, met ASCII-spaties eromheen weggelaten; een lege of blanke status en `null` tellen als Standard, net als een ontbrekende, zonder waarschuwing. Een `imageDigest` met een spatie of tab is een fout. Een ontbrekende digest wordt getoond als `(none)`; een digest die zelf `(none)` is, of met een aanhalingsteken begint, wordt tussen aanhalingstekens getoond, zodat de twee niet verward kunnen worden. Stuur-, regeleinde- en onzichtbare tekens in een waarde, bestandsnaam of opdrachtregelargument dat een melding herhaalt, worden als `\u{hex}` geschreven in tekst en meldingen, en als `\uXXXX`-escape in `--json`-strings; de vier prijsbestandteksten in `--json` (`source_url`, `source_date`, `region`, `region_name`) worden eerst in de tekstvorm geschreven (`\u{202e}`, `\n`), dus in de JSON staan ze met een dubbele backslash (`\\u{202e}`). Een gebruiksfout blijft daardoor op de regels die clap afdrukt. Fouten over een heel bestand noemen het bestand; fouten over één image zeggen `file N: image M`.

`imagePushedAt` is een JSON-getal van seconden sinds 1970-01-01 UTC, van 0 tot 9223372036854775807 (een breuk wordt afgekapt en een exponent is toegestaan), of een ISO 8601-string `YYYY-MM-DDTHH:MM:SS` met optionele breuk en een zone, `Z` of `+HH:MM` of `-HH:MM` tot 23:59; `T` en `Z` mogen klein. Een ontbrekende of `null`-waarde slaat de image voor leeftijd over en geeft een waarschuwing. Al het andere is `invalid imagePushedAt`: een string van cijfers, een datum zonder tijd, een tijd zonder zone, een spatie in plaats van `T`, seconden 60, uur 25, een offset als `+0100`, een negatief getal of een getal boven die grens.

Prijsbestand. `--prices` leest een JSON-bestand zoals `prices/ecr-storage.json`: `source_url` (begint met `https://`), `source_date`, `region`, `region_name`, `usd_per_gb_month` met `standard`, `archive_first_150_tb` en `archive_over_150_tb`, `usd_per_gb_retrieval`, `archive_tier_tb` en `archive_min_days`. Elk tarief is een string van cijfers met hoogstens één decimaalpunt tussen cijfers, hoogstens 40 tekens, met ASCII-spaties eromheen weggelaten: `"1e2"`, `"-0.10"`, `".5"`, `"5."` en een getal zijn `price file has a rate that is not a decimal`. Een tarief dat het decimale type niet exact kan bevatten wordt geweigerd, omdat het zou afronden. Het type bevat hoogstens 28 cijfers (nullen vóór het eerste en na het laatste cijfer tellen niet, nullen direct na de punt wel) en niets boven 79228162514264337593543950335: een tarief dat zou afronden is `price file has a rate with more than 28 digits`. Een afwezig veld is `price file is missing NAME`; een aanwezig veld van het verkeerde soort of buiten bereik is `price file has an invalid NAME`. Een bestand dat geen gewoon bestand is, is een fout, net als bij een invoerbestand.

Exacte bedragen. Geld is decimaal, nooit floating point, en een bedrag wordt nooit vóór de cent afgerond. Een tarief heeft hoogstens 28 cijfers; de kosten van een repository zijn haar bytes maal het tarief gedeeld door 1.073.741.824, het aandeel van een repository in een getrapt archiefbedrag is dat bedrag maal haar gearchiveerde bytes gedeeld door alle gearchiveerde bytes, en elk daarvan wordt als exacte breuk in geheeltallige rekenkunde (512 bits) bewaard en exact opgeteld. De cent wordt één keer aan het eind genomen, half weg van nul, voor elke rij en voor het totaal. De fout `cost does not fit` blijft bestaan voor een getal dat niet in 512 bits past. Tijd en geheugen groeien mee met de invoer; de gemeten waarden staan in de Engelse README (Limitations, Exact amounts en Memory and size).

Bereiken. `--keep` is 1 tot 10000. `--older-than-days` en `--untagged-days` zijn 1 tot 36500. `--as-of` is 0000-01-01 tot 9999-12-31, als echte kalenderdatum. Een waarde buiten het bereik is een gebruiksfout met afsluitcode 2. Een `+` vooraan wordt geaccepteerd in een getaloptie (`--keep +5`), zoals Rust het leest.

Geheugen en grootte. Een bestand boven 268435456 bytes wordt geweigerd, maar de limiet geldt per bestand, niet per run, en de geparste JSON is groter dan het bestand. Elk bestand wordt gelezen voordat er een wordt geparsed, dus het geheugen van meerdere bestanden telt op. Er is geen vaste factor: het hangt van de vorm af. De gemeten factoren per vorm staan in de Engelse README.

Platforms. Gebouwd en getest op Linux x86_64 met glibc (de binary heeft glibc 2.34 of nieuwer nodig) en ook gebouwd en gedraaid met het musl-target. Windows en macOS zijn voor deze versie niet gedraaid. De controle die een gesloten standaarduitvoer (`>&-`) tot afsluitcode 2 maakt bestaat alleen op Linux. Op Linux geven een volle device (`>/dev/full`), een gesloten pipe en een standaarduitvoer die alleen leesbaar is (`1<file`) ook afsluitcode 2, met de melding `ecr-bill: could not write output`. `>/dev/null` en `1<>/dev/null` geven 0.

## Prijzen en verantwoordelijkheid

De ingebouwde prijzen zijn een momentopname van <https://aws.amazon.com/ecr/pricing/>, gelezen op 2026-11-09. Controleer de actuele prijzen zelf. Besluiten op basis van deze uitvoer zijn van jou.

`--prices` levert je eigen prijsbestand.

Oude prijzen worden getoond, niet verborgen. Is de `source_date` van het gebruikte prijsbestand (het ingebouwde of dat van `--prices`) meer dan 180 dagen vóór `--as-of`, of vóór vandaag als `--as-of` ontbreekt, dan gaat er één regel naar standaardfout en blijft de afsluitcode 0: `warning: prices are N days old (price date YYYY-MM-DD, as of YYYY-MM-DD); check current prices`. Precies 180 dagen geeft geen waarschuwing. De standaarduitvoer is hetzelfde met of zonder de waarschuwing (hij staat niet in de `warnings`-lijst). De klok wordt alleen voor deze controle gelezen, nooit om de rekening te dateren. Een `source_date` die niet als `YYYY-MM-DD` is geschreven geeft geen waarschuwing en geen fout, en een prijsdatum na de as-of-dag ook niet.

## Onderhoud

Meld een fout via GitHub Issues in deze repository. Na een release is deze tool rustend. Een nieuwe versie verschijnt alleen voor een gemelde fout, een verzoek of een beveiligingsprobleem. Een beveiligingsprobleem gaat voor.

## Ondersteuning

Meld een fout via GitHub Issues in deze repository.

## Licentie

MIT OR Apache-2.0. Zie [LICENSE-MIT](LICENSE-MIT) en [LICENSE-APACHE](LICENSE-APACHE).

De licenties van de externe crates in het programma staan in [THIRD-PARTY-LICENSES.html](THIRD-PARTY-LICENSES.html).

## Contact

PEPAI, pep_ai@icloud.com, <https://www.pepai.nl>. X: <https://x.com/pepai_nl>. Ko-fi: <https://ko-fi.com/pepai>.
