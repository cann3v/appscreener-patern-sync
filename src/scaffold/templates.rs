use crate::scaffold::model::RuleScaffoldSpec;

pub fn readme(spec: &RuleScaffoldSpec) -> String {
    let heading = spec.document_heading();

    format!(
        r#"# {heading}

Комплект пользовательского правила Solar AppScreener.

Статус разработки:

```text
черновик
```

## Назначение правила

TODO: описать обнаруживаемую проблему и её влияние на безопасность.

## Целевые языки

TODO.

## Целевые платформы

TODO.

## Состав

- `rule/rule-metadata.md` — метаданные правила;
- `rule/patterns/` — XML-паттерны и manifest;
- `tests/include/` — автономные заголовочные файлы;
- `tests/src/` — положительные и отрицательные тесты;
- `tests/build.sh` — сборка тестового проекта;
- `docs/pattern-catalog.md` — каталог паттернов и границы обнаружения.

## Планируемые паттерны

TODO.

## Сборка тестов

```bash
chmod +x tests/build.sh
./tests/build.sh
```

Каждый `.c` и `.cc` компилируется в отдельный объектный файл.

## Контрольный baseline

Solar AppScreener ещё не запускался.

## Ограничения

TODO.
"#
    )
}

pub fn rule_metadata(spec: &RuleScaffoldSpec) -> String {
    let heading = spec.document_heading();

    let title = markdown_table_cell(spec.display_title());

    let cwe = spec.cwe_label();

    let analysis_type = spec.pattern_type.analysis_label();

    let pattern_type = spec.pattern_type.as_manifest_value();

    format!(
        r#"# Метаданные правила {heading}

| Поле | Значение |
|---|---|
| Rule ID | `TODO` |
| CWE | {cwe} |
| Название | {title} |
| Языки | TODO |
| Поддерживаемая платформа | TODO |
| Тип анализа | {analysis_type} |
| Версия Solar AppScreener | TODO |
| Статус | Черновик |

## Описание

TODO.

## Рекомендация

TODO.

## Параметры регистрации

| Параметр | Значение |
|---|---|
| Pattern type | `{pattern_type}` |
| Severity | `{severity}` |
| Confidence | `{confidence}` |
| Active | `true` |

## Подтверждённое покрытие

Пока отсутствует.

## Результат контрольного сканирования

Контрольное сканирование не выполнялось.

## Ограничения

TODO.
"#,
        severity = spec.severity,
        confidence = spec.confidence,
    )
}

pub fn pattern_catalog(spec: &RuleScaffoldSpec) -> String {
    let heading = spec.pattern_catalog_heading();

    format!(
        r#"# {heading}

Паттерны ещё не зарегистрированы.

## Планируемое покрытие

TODO.

## Реализованные паттерны

Пока отсутствуют.

## Положительные тесты

Пока отсутствуют.

## Отрицательные тесты

Пока отсутствуют.

## Известные false positive

Пока отсутствуют.

## Известные false negative

Пока отсутствуют.

## Результат контрольного сканирования

Контрольное сканирование не выполнялось.

## Ограничения

TODO.
"#
    )
}

pub fn patterns_manifest(spec: &RuleScaffoldSpec) -> String {
    format!(
        r#"version: 1

defaults:
  type: {pattern_type}
  severity: {severity}
  confidence: {confidence}
  active: true

patterns: {{}}
"#,
        pattern_type = spec.pattern_type.as_manifest_value(),
        severity = spec.severity,
        confidence = spec.confidence,
    )
}

pub fn build_script() -> &'static str {
    r#"#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source_dir="$project_dir/src"
include_dir="$project_dir/include"
build_dir="$project_dir/build"

c_compiler="${CC:-cc}"
cpp_compiler="${CXX:-c++}"

mkdir -p "$build_dir"

shopt -s nullglob

c_sources=("$source_dir"/*.c)
cpp_sources=("$source_dir"/*.cc)

for source_file in "${c_sources[@]}"; do
    source_name="$(basename -- "${source_file%.c}")"

    "$c_compiler" \
        -x c \
        -std=c11 \
        -Wall \
        -Wextra \
        -I"$include_dir" \
        -c "$source_file" \
        -o "$build_dir/$source_name.c.o"
done

for source_file in "${cpp_sources[@]}"; do
    source_name="$(basename -- "${source_file%.cc}")"

    "$cpp_compiler" \
        -x c++ \
        -std=c++17 \
        -Wall \
        -Wextra \
        -I"$include_dir" \
        -c "$source_file" \
        -o "$build_dir/$source_name.cc.o"
done

echo "Build completed:"
echo "  C objects:   ${#c_sources[@]}"
echo "  C++ objects: ${#cpp_sources[@]}"
echo "  Output:      $build_dir"
"#
}

fn markdown_table_cell(value: &str) -> String {
    value.replace('|', r"\|")
}
