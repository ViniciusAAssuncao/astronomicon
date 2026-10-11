# Fase 2 — sessão Rust hospedável

`rocketcon_sim::RocketconSession` abre um save existente, seleciona um veículo com estado físico, mantém o comando atual e avança o tick real pelo caminho transacional medido na Fase 1. O banco continua sendo a fonte da verdade. Cada `step` grava o estado e seus efeitos na transação atômica já existente. Um erro de tick preserva o snapshot e o histórico em memória.

## Contrato atual

- `load(path, vehicle_id)` exige um arquivo existente, aplica as migrações e carrega o estado físico inicial.
- `apply_control(VehicleControlInput)` substitui o controle usado nos próximos ticks. O controle não é persistido no save.
- `step(dt_seconds)` exige tempo positivo e finito, executa um tick e publica o novo `FlightSnapshot`.
- `snapshot()` fornece a última leitura. Antes do primeiro tick, campos derivados da aerodinâmica, carga G e contato são `None`.
- `trajectory()` fornece posições amostradas durante **esta sessão**, incluindo a inicial. Não é uma previsão orbital nem o histórico completo do save.
- `events()` fornece transições de atmosfera e contato observadas entre ticks **desta sessão**. Não reconstrói eventos anteriores ao carregamento.
- `save()` solicita um checkpoint passivo do WAL. O tick já está confirmado antes de `step` retornar; o checkpoint apenas tenta incorporar páginas do WAL ao arquivo principal.
- `close()` encerra as conexões de forma ordenada.

O snapshot usa valores escalares em SI e identificadores UUID; não expõe `SqlitePool`, componentes internos ou tipos de domínio à futura interface. A API é assíncrona em Rust. O futuro adaptador FFI esconderá o runtime Tokio do C++.

## CLI

Um save de benchmark pode ser gerado com `cargo run -p rocketcon-sim --bin flight_tick_bench -- 1`. Para preservar esse save durante a demonstração, faça uma cópia e execute:

```text
cargo run -p rocketcon-sim --bin flight_session -- saves/session-demo.db e2897c6a-7d04-4ebc-882c-87984a74a200 3 0.02 0 0 0
```

Os argumentos após o UUID são quantidade de ticks, `dt` em segundos, pitch, yaw e roll. O CLI usa somente a fachada da sessão para dirigir o veículo e imprime telemetria de cada tick, quantidade de amostras e eventos.

## Verificação

- `cargo test -p rocketcon-sim --bin flight_tick_bench`: três testes passaram, incluindo tick atômico, snapshot climático e carregar → controlar → avançar → salvar → recarregar.
- `cargo check -p rocketcon-sim --all-targets`: passou.
- CLI executado em cópia local de `bench-atmosphere.db` por três ticks; época, posição, velocidade, altitude, Mach e carga G foram publicados.

O passo seguinte é estabilizar os DTOs e criar o crate `rocketcon-ffi` com uma superfície pequena. A trajetória amostrada e os eventos transitórios exigirão política de retenção antes de uma sessão de longa duração em tempo real.
