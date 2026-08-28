# Solana Level 1 Token Starter

Учебный starter для итоговых заданий первого уровня курса Superteam KZ. Он показывает современный минимальный каркас токен-программы без привязки к legacy JavaScript SDK.

> Это исходная точка, а не готовое решение. Не работайте напрямую в ветке `main`: для каждого задания создавайте отдельную ветку.

## Как получить проект через GitHub

Если вы ещё не работали с GitHub:

1. Нажмите **Fork** в правом верхнем углу страницы и создайте копию репозитория в своём аккаунте.
2. На странице своей копии нажмите **Code** и скопируйте HTTPS-ссылку.
3. Выполните в терминале:

   ```bash
   git clone <ссылка-на-ваш-fork>
   cd education
   git checkout -b task/01-tests
   ```

4. После выполнения задания сохраните изменения:

   ```bash
   git add .
   git commit -m "Complete task 01 tests"
   git push -u origin task/01-tests
   ```

5. Отправьте преподавателю ссылку на ветку `task/01-tests` или на последний commit.

Не знаете Git? Для этих заданий достаточно операций `clone`, `checkout -b`, `add`, `commit` и `push`; команды выше можно использовать как готовый сценарий.

## Задание 1 — покрыть токен-программу тестами

В проекте уже есть минимальный LiteSVM-тест `create_token`. Его нужно усилить и добавить тесты остальных реализованных инструкций.

### Что нужно сделать

- В тесте `create_token` проверить `decimals`, mint authority, supply и владельца mint, а не только наличие аккаунта.
- Покрыть `create_token_account`: проверить владельца token account, mint и token program.
- Покрыть `mint_tokens`: проверить изменение баланса получателя и общего supply.
- Покрыть `transfer_tokens`: проверить оба баланса и неизменность общего supply.
- Добавить негативные сценарии: нулевая сумма, неверный authority, другой mint и одинаковые source/destination.
- Обновить README в своём fork: указать версии, команды запуска и кратко описать добавленные тесты.

### Готовность задания

Чистый checkout вашей ветки должен проходить:

```bash
anchor build --ignore-keys
cargo test --workspace --locked
```

Флаг `--ignore-keys` нужен только потому, что локальный program keypair намеренно не хранится в учебном репозитории. Для собственного devnet-деплоя создайте keypair локально и синхронизируйте ID командой `anchor keys sync`, но не добавляйте файл keypair в Git.

Не публикуйте keypair, seed phrase, приватные ключи или `.env` с секретами.

Следующие задания выполняются в ветках `task/02-burn` и `task/03-escrow`. Их условия выдаются на учебной платформе; готовой реализации в starter нет.

## Зафиксированный стек

- Anchor CLI и crates: `1.1.2`
- Solana CLI: `3.1.10`
- Rust: `1.89.0`
- тесты программ: Rust + LiteSVM `0.10.0`
- токены: `anchor_spl::token_interface`, совместимый с Token Program и Token-2022
- рекомендуемый клиент для нового TypeScript-кода: `@solana/kit`

`@solana/web3.js` относится к legacy-стеку. TypeScript-клиент Anchor `@anchor-lang/core` по-прежнему зависит от `@solana/web3.js` v1, поэтому в этом starter тесты написаны на Rust и LiteSVM. Для нового клиентского приложения используйте `@solana/kit`, если задание явно не требует другого.

Оригинальный Token Program остается рабочим и широко используется. Для новых токенов в учебных заданиях используйте Token-2022, а program-код пишите через `token_interface`, чтобы сохранить совместимость с обоими Token Program.

## Что уже реализовано

- создание mint с выбранной token-программой;
- создание associated token account;
- выпуск токенов через `mint_to`;
- перевод через `transfer_checked`;
- сжигание токенов через `burn_checked` с `decimals` из mint;
- проверки положительной суммы, полномочий, mint и token program на уровне Anchor accounts constraints;
- воспроизводимый набор Rust + LiteSVM-тестов для всех реализованных инструкций.

В ветке `task/03-escrow` дополнительно реализована отдельная программа escrow.

## Быстрый старт

1. Установите версии из раздела «Зафиксированный стек» через AVM, rustup и официальный Solana installer.
2. Для локального прохождения заданий выполните `anchor build --ignore-keys`. Для собственного devnet-деплоя создайте локальный program keypair и выполните `anchor keys sync`. Не коммитьте keypair или seed phrase.
3. После первой сборки выполните `cargo test --workspace --locked`.
4. Разрабатывайте каждое задание в отдельной ветке: `task/01-tests`, `task/02-burn`, `task/03-escrow`.

Тест загружает собранный файл `target/deploy/solana_level_1_token_starter.so`, поэтому перед первым `cargo test` нужен `anchor build --ignore-keys`.

## Выполненное задание `task/01-tests`

Решение рассчитано на зафиксированный стек: Anchor CLI/crates `1.1.2`, Solana CLI `3.1.10`, Rust `1.89.0` и LiteSVM `0.10.0`. Все тестовые mint создаются через Token-2022, program-код использует `anchor_spl::token_interface`, а перевод выполняется только через `transfer_checked`. Новый TypeScript-код в решении отсутствует.

Полный запуск из чистого checkout:

```bash
anchor --version
solana --version
rustc --version
anchor build --ignore-keys
cargo test --workspace --locked
```

Ожидаемые версии первых трёх команд — `anchor-cli 1.1.2`, `solana-cli 3.1.10` и `rustc 1.89.0`. Ожидаемый результат сборки — успешное создание `target/deploy/solana_level_1_token_starter.so`; результат интеграционного набора — `8 passed; 0 failed`.

Тесты проверяют:

- параметры Token-2022 mint: program owner, `decimals`, mint authority и начальный `supply`;
- program owner и поля `owner`, `mint`, `amount` созданного associated token account;
- точные изменения баланса и `supply` после `mint_tokens`;
- оба баланса и неизменность `supply` после `transfer_tokens`;
- отказ и отсутствие изменений состояния при нулевой сумме, неверном authority, token account другого mint и совпадающих source/destination.

## Выполненное задание `task/02-burn`

Инструкция `burn_tokens` использует `anchor_spl::token_interface::burn_checked` и передаёт в CPI значение `decimals`, прочитанное из проверенного mint. Критичные аккаунты типизированы: `authority` — `Signer`, `mint` — `InterfaceAccount<Mint>`, исходный token account — `InterfaceAccount<TokenAccount>`, token program — `Interface<TokenInterface>`. `UncheckedAccount` не используется.

Anchor account constraints до выполнения CPI проверяют:

- подпись владельца token account через `Signer` и `token::authority = authority`;
- связь token account с mint через `token::mint = mint`;
- соответствие mint и token account переданному token program через `mint::token_program` и `token::token_program`;
- положительную сумму сжигания через программную ошибку `AmountMustBePositive`.

Команды для чистого checkout остаются теми же:

```bash
anchor build --ignore-keys
cargo test --workspace --locked
```

Ожидаемый результат — успешная сборка и `13 passed; 0 failed`. Позитивный тест подтверждает одинаковое уменьшение баланса token account и общего `supply`. Негативные тесты покрывают нулевую сумму, неверный authority, другой mint и недостаточный баланс; после каждого отказа отдельно проверяется неизменность баланса и `supply`.

## Выполненное задание `task/03-escrow`

В workspace добавлена программа `programs/escrow` — минимальный escrow для Token-2022. Для каждой сделки создаются два уникальных PDA:

- state PDA: `[b"escrow", sender, deal_id.to_le_bytes()]`;
- vault PDA: `[b"vault", escrow_state]`.

`EscrowState` хранит `sender`, `receiver`, `mint`, точную `amount`, `deal_id`, canonical bump и статус. Vault создаётся для конкретной сделки, принадлежит выбранной token program, связан с сохранённым mint, а его token authority — state PDA этой сделки. Общего vault между сделками нет.

### State machine

```text
initialize: отсутствует -> Created
deposit:    Created     -> Funded
release:    Funded      -> Released -> vault и state закрыты
cancel:     Created     -> Cancelled -> vault и state закрыты
cancel:     Funded      -> Cancelled -> токены возвращены, vault и state закрыты
```

`deposit` переводит ровно записанную сумму. `release` и `cancel` сначала опустошают vault через `anchor_spl::token_interface::transfer_checked` с `decimals` проверенного mint, затем закрывают Token-2022 vault через CPI `close_account`. Rent vault и state возвращается `sender`. Закрытие терминальных аккаунтов исключает повторное завершение существующей сделки.

### Account constraints и threat model

Программа не доверяет клиенту и не использует `UncheckedAccount` для критичных аккаунтов. Anchor constraints и проверки handler защищают следующие инварианты:

- `sender` обязан подписать транзакцию и совпадать с `EscrowState.sender`;
- state PDA повторно выводится из сохранённых `sender`, `deal_id` и bump;
- receiver и mint обязаны совпадать с полями state;
- mint, source/destination и vault обязаны принадлежать одной переданной token program;
- source/destination проверяются как token accounts нужного mint и authority, а destination release — canonical ATA receiver;
- vault повторно выводится из state PDA и обязан иметь authority этого PDA;
- запрещены нулевая сумма, одинаковые sender/receiver, повторный deposit и неверный status;
- deposit принимает только точную сумму сделки, release требует точный баланс vault, cancel возвращает весь фактический остаток;
- ошибка любого constraint или CPI атомарно откатывает изменения состояния.

Таким образом, подмена signer, mint, receiver, token account, token program, PDA или status не позволяет вывести или заблокировать чужие токены. Недостаточный баланс отклоняется Token-2022 CPI без частичного изменения state.

### Сборка и тесты

Используется зафиксированный стек: Anchor CLI/crates `1.1.2`, Solana CLI `3.1.10`, Rust `1.89.0`, LiteSVM `0.10.0` и Token-2022. TypeScript-код и legacy `@solana/web3.js` не добавлялись.

Из чистого checkout ветки выполните:

```bash
anchor --version
solana --version
rustc --version
anchor build --ignore-keys
cargo test --workspace --locked
```

Ожидаемые версии — `anchor-cli 1.1.2`, `solana-cli 3.1.10`, `rustc 1.89.0`. Ожидаемый результат — успешная сборка двух `.so` и все `21` интеграционных теста пройдены: `13` token/burn и `8` escrow.

Escrow LiteSVM-тесты включают два положительных end-to-end сценария (`release` и funded `cancel`) и отказы для нулевой суммы, одинаковых участников, повторного `deal_id`, неверного signer, недостаточного баланса, подмены mint/receiver, повторного deposit и повторного release/cancel. После каждого отказа проверяется неизменность соответствующих state, vault, token balances или supply.

## Правила сдачи

- сдавайте публичную ссылку на GitHub-репозиторий и указывайте ветку или commit SHA;
- добавьте в README команды сборки и тестирования, ожидаемый результат и краткое описание архитектуры;
- не добавляйте в репозиторий private keys, seed phrases, `.env` с секретами или файлы keypair;
- не используйте `@solana/web3.js` в новом клиентском коде;
- для переводов токенов используйте `transfer_checked`, а не unchecked transfer;
- не подменяйте проверки полномочий только клиентской логикой: все критичные инварианты должны проверяться программой.

## Что считается современным решением

Современность здесь определяется не только номером версии. Решение должно использовать строгие account constraints, проверяемые state transitions, Token-2022 для нового токена, `token_interface` для совместимости, `transfer_checked` для переводов и воспроизводимые LiteSVM-тесты. Если официальные стабильные рекомендации Solana или Anchor изменятся, студент должен зафиксировать выбранные версии и объяснить отклонение в README.
