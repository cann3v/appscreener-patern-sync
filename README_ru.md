# appScreener Pattern Sync

CLI-утилита на Rust для синхронизации локальных XML-паттернов с существующим кастомным правилом Solar appScreener.

Локальная директория является источником истины: после успешного применения правило будет содержать ровно тот набор паттернов, который находится в указанной директории.

Утилита не создаёт правила и не изменяет их метаданные. `PUT /rules/custom` не используется.

## Возможности

- безопасный режим предварительного просмотра `plan`;
- полное зеркалирование состава паттернов;
- создание новых паттернов;
- обновление существующих паттернов с сохранением UUID;
- удаление паттернов, отсутствующих локально;
- проверка XML-фрагментов до обращения к серверу;
- обязательная критичность и уверенность;
- сохранение серверного snapshot перед изменениями;
- повторная проверка состояния после загрузки и удаления;
- структурированное логирование через `tracing`;
- JWT из переменной окружения;
- защита от случайного удаления всех паттернов.

## Требования

- Rust 1.85 или новее;
- доступ к API Solar appScreener;
- JWT пользователя с правами на изменение кастомных правил;
- существующий UUID кастомного правила.

## Сборка

```powershell
cargo build --release
```

Исполняемый файл:

```text
target\release\appscreener-pattern-sync.exe
```

Проверка проекта:

```powershell
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Структура локальной директории

```text
patterns/
├── patterns.yaml
├── P003-sensitive-source-arg2.xml
├── P004-sensitive-source-arg3.xml
├── P005-sensitive-source-arg4.xml
├── P100-sensitive-free-arg0-sink.xml
└── P200-sensitive-memory-sanitizer-arg0.xml
```

По умолчанию серверное имя паттерна формируется из имени XML-файла без расширения:

```text
P003-sensitive-source-arg2.xml
```

превращается в:

```text
P003-sensitive-source-arg2
```

## Конфигурация

По умолчанию утилита ищет manifest:

```text
<patterns-dir>\patterns.yaml
```

Другой путь можно передать через:

```text
--manifest <PATH>
```

### Минимальный конфиг

```yaml
version: 1

defaults:
  type: DATAFLOW
  severity: 3
  confidence: 1
  active: true

patterns: {}
```

Эти параметры применяются ко всем XML-файлам директории.

`severity` и `confidence` обязательны. На исследованной версии appScreener паттерн без этих параметров сохраняется в базе, но не участвует в анализе.

### Полный конфиг

```yaml
version: 1

defaults:
  type: DATAFLOW
  severity: 3
  confidence: 1
  active: true

patterns:
  P003-sensitive-source-arg2:
    name: Windows-sensitive-password-source-arg2
    severity: 3
    confidence: 2

  P004-sensitive-source-arg3:
    severity: 2
    confidence: 1
    active: true

  P005-sensitive-source-arg4.xml:
    type: DATAFLOW
    severity: 3
    confidence: 1
    active: true
    fileRegex: '.*\.(c|cc|cpp|cxx)$'

  reporting-pattern:
    type: REPORTING
    queryType: REGEX
    severity: 2
    confidence: 1
    active: true
    fileRegex: '.*\.cpp$'
```

Ключом в `patterns` может быть:

- имя файла без `.xml`;
- полное имя файла с `.xml`.

Нельзя одновременно задавать оба варианта для одного файла.

## Параметры паттерна

| Параметр | Обязательный | Значения | Описание |
|---|---:|---|---|
| `name` | Нет | Строка | Переопределяет имя файла без расширения |
| `type` | Да | `DATAFLOW`, `REPORTING` | Тип паттерна appScreener |
| `severity` | Да | `0..3` | Уровень критичности |
| `confidence` | Да | Целое число | Уровень уверенности |
| `active` | Нет | `true`, `false` | По умолчанию `true` |
| `queryType` | Нет | `REGEX`, `XPATH` | Тип запроса |
| `fileRegex` | Нет | Строка | Фильтр анализируемых файлов |

`type` и `queryType` — разные поля API:

```text
type:      DATAFLOW | REPORTING
queryType: REGEX | XPATH
```

Для XML DataFlow-паттернов поле `queryType` обычно не задаётся.

Значения конкретного паттерна имеют приоритет над `defaults`.

## XML-фрагменты

DataFlow DSL appScreener может содержать несколько верхнеуровневых секций:

```xml
<condition>
    <!-- ... -->
</condition>

<taintFlowChain>
    <!-- ... -->
</taintFlowChain>
```

Это не обычный XML-документ с единственным корневым элементом.

Для проверки утилита временно оборачивает содержимое техническим корнем. В appScreener передаётся исходный фрагмент без этого корня.

Запрещены:

- пустые XML-файлы;
- некорректно закрытые элементы;
- XML declaration;
- `DOCTYPE`.

## Авторизация

Рекомендуемый способ — переменная окружения:

```powershell
$env:APPSCREENER_TOKEN = "<JWT>"
```

Не рекомендуется передавать JWT через `--token`, поскольку он может попасть в историю команд и список процессов.

Токен не записывается:

- в snapshot;
- в лог;
- в план;
- в сообщения об HTTP-ошибках.

## Режим plan

`plan` выполняет только читающие запросы и не изменяет appScreener:

```powershell
target\release\appscreener-pattern-sync.exe plan `
  --base-url http://appscreener.example `
  --rule-id <RULE_UUID> `
  --patterns-dir C:\path\to\patterns
```

Пример результата:

```text
ACTION   PATTERN                                       DETAILS
-------- --------------------------------------------- ------------------------------
CREATE   P003-sensitive-source-arg2                    not present on server
UPDATE   P004-sensitive-source-arg3                    XML changed
SKIP     P005-sensitive-source-arg4                    already synchronized
DELETE   obsolete-pattern                             not present in local directory

Summary: create=1, update=1, skip=1, delete=1
```

Значения:

- `CREATE` — локального паттерна нет на сервере;
- `UPDATE` — паттерн найден по имени, но его параметры или XML отличаются;
- `SKIP` — паттерн уже синхронизирован;
- `DELETE` — серверного паттерна нет в локальной директории.

Имена сопоставляются без учёта регистра.

Если на сервере найдено несколько паттернов с одинаковыми именами без учёта регистра, операция завершается ошибкой.

## Режим apply

Перед применением рекомендуется внимательно проверить результат `plan`.

```powershell
target\release\appscreener-pattern-sync.exe apply `
  --base-url http://appscreener.example `
  --rule-id <RULE_UUID> `
  --patterns-dir C:\path\to\patterns `
  --snapshot-out .\before-import.snapshot.json
```

Snapshot обязателен и не должен существовать до запуска. Утилита намеренно не перезаписывает существующие snapshot-файлы.

## Порядок применения

Операции выполняются в безопасном порядке:

1. обновление существующих паттернов через `PUT`;
2. создание новых паттернов через `POST`;
3. финализация каждого созданного паттерна через `PUT`;
4. проверка наличия всего локального набора;
5. удаление лишних серверных паттернов;
6. финальная проверка точного состава.

Созданный паттерн дополнительно сохраняется через `PUT`, поскольку appScreener использует этот запрос для окончательной регистрации паттерна в анализаторе.

Если `POST` или финализирующий `PUT` завершится ошибкой, старые паттерны ещё не будут удалены.

## Snapshot

Snapshot содержит исходный серверный набор:

- UUID;
- имя;
- XML;
- `ruleId`;
- `type`;
- `severity`;
- `confidence`;
- `active`;
- `shared`;
- `user`;
- `queryType`;
- `fileRegex`.

Пример структуры:

```json
{
  "version": 1,
  "ruleId": "<RULE_UUID>",
  "patterns": [
    {
      "uuid": "<PATTERN_UUID>",
      "ruleId": "<RULE_UUID>",
      "severity": 3,
      "confidence": 1,
      "name": "example-pattern",
      "xml": "<condition>...</condition>",
      "type": "DATAFLOW",
      "active": true,
      "shared": false,
      "user": "username"
    }
  ]
}
```

Автоматическая команда восстановления пока не реализована. Удалённый паттерн можно пересоздать по snapshot, но он может получить новый UUID.

## Защита от пустой директории

По умолчанию утилита отказывается удалять весь серверный набор, если локальная директория не содержит XML:

```text
local directory contains no XML patterns
```

Для намеренной очистки правила требуется:

```text
--allow-empty
```

Пример:

```powershell
target\release\appscreener-pattern-sync.exe apply `
  --base-url http://appscreener.example `
  --rule-id <RULE_UUID> `
  --patterns-dir C:\path\to\empty-patterns `
  --snapshot-out .\before-cleanup.snapshot.json `
  --allow-empty
```

## Логирование

По умолчанию используется уровень `INFO`.

```powershell
appscreener-pattern-sync.exe plan ...
```

Расширенное логирование:

```powershell
appscreener-pattern-sync.exe -v plan ...
```

Максимальная детализация:

```powershell
appscreener-pattern-sync.exe -vv plan ...
```

Только ошибки:

```powershell
appscreener-pattern-sync.exe -q plan ...
```

Также поддерживается `RUST_LOG`:

```powershell
$env:RUST_LOG = "appscreener_pattern_sync=debug"
```

План выводится в `stdout`, технические логи — в `stderr`:

```powershell
appscreener-pattern-sync.exe plan ... > plan.txt
```

XML целиком не выводится в лог. Для сравнения используются размер и SHA-256.

## Используемые методы API

```text
GET    /rules/custom/{id}/info
GET    /rules/custom/{ruleId}/patterns
POST   /patterns/pattern
PUT    /patterns/pattern
DELETE /patterns/pattern?uuid={uuid}
```

Метаданные правила не изменяются:

```text
PUT /rules/custom
```

не вызывается.

## Проверка после импорта

После успешного `apply` повторно запустите `plan`:

```powershell
target\release\appscreener-pattern-sync.exe plan `
  --base-url http://appscreener.example `
  --rule-id <RULE_UUID> `
  --patterns-dir C:\path\to\patterns
```

Ожидаемый результат:

```text
Summary: create=0, update=0, skip=5, delete=0

The rule already matches the local directory.
```

В ответе appScreener каждый рабочий паттерн должен содержать как минимум:

```json
{
  "severity": 3,
  "confidence": 1,
  "type": "DATAFLOW",
  "active": true
}
```

## Диагностика

### HTTP 401

JWT отсутствует, истёк или был отозван:

```text
check APPSCREENER_TOKEN
```

Создайте новый JWT и обновите переменную окружения.

### HTTP 403

Пользователь не имеет прав на изменение правила.

### HTTP 500

Утилита выводит ограниченное тело ответа appScreener. Проверьте:

- обязательные `severity` и `confidence`;
- допустимое значение `type`;
- корректность XML DSL;
- принадлежность UUID кастомному правилу.

### Snapshot уже существует

Утилита не перезаписывает резервные копии:

```text
the file must not already exist
```

Укажите новое имя:

```text
--snapshot-out .\before-import-02.snapshot.json
```

### Паттерн существует, но не участвует в анализе

Проверьте, что серверный ответ содержит:

```json
{
  "severity": 3,
  "confidence": 1,
  "active": true
}
```

Новый паттерн должен пройти оба запроса:

```text
POST /patterns/pattern
PUT  /patterns/pattern
```

Второй запрос соответствует кнопке «Сохранить» в UI appScreener.

## Ограничения

- API appScreener не предоставляет пакетную транзакцию;
- автоматический rollback пока не реализован;
- при восстановлении удалённых паттернов их UUID могут измениться;
- утилита работает только с существующим кастомным правилом;
- создание и изменение метаданных правила не поддерживается;
- поддерживаются локальные файлы `*.xml`;
- диапазон `confidence` не описан в OpenAPI и контролируется конфигурацией.