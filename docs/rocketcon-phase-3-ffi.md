# Fase 3 — fronteira CXX

O crate `rocketcon-ffi` usa `#[cxx::bridge]` conforme a documentação oficial do CXX. O motor é um tipo Rust opaco entregue ao C++ por `rust::Box<rocketcon::Engine>`. `FfiVec3` e `FlightSnapshot` são tipos compartilhados gerados pelo CXX. Tokio, `RocketconSession`, SQLx e os tipos de domínio permanecem no lado Rust.

O build produz `target/debug/rocketcon_ffi.lib` no Windows MSVC e gera o cabeçalho `target/cxxbridge/rocketcon-ffi/src/lib.rs.h`. O consumidor C++ inclui esse cabeçalho e usa `target/cxxbridge` como diretório de includes. O cabeçalho é gerado, não editado manualmente.

## Contrato

- `create_engine()` cria o motor; erros atravessam a fronteira como `rust::Error`.
- `Engine::load_save(path, vehicle_uuid)` carrega um save existente e preserva a sessão anterior se a nova carga falhar.
- `Engine::set_control(pitch, yaw, roll)` substitui os comandos de atitude dos próximos ticks. Não representa um throttle global.
- `Engine::step(dt_seconds)` executa o tick real; `dt_seconds` deve ser positivo e finito.
- `Engine::snapshot()` retorna uma cópia da telemetria em unidades SI. Cada campo opcional tem um booleano `has_*` correspondente. Antes do primeiro tick, os valores derivados estão ausentes.
- `Engine::save()` pede um checkpoint passivo do WAL; cada tick já está confirmado ao retornar de `step`.
- A destruição do `rust::Box` encerra as conexões da sessão.

O CXX converte `Result<T>` de Rust em exceção `rust::Error` no C++. Os métodos são síncronos para o consumidor; o runtime Tokio fica dentro do motor. Um motor deve ser usado por apenas uma thread por vez.

## Verificação

- `cargo check -p rocketcon-ffi` passou.
- `cargo test -p rocketcon-ffi --test boundary` passou: save temporário, erro antes do load, `dt` inválido, tick, DTO, tentativa de novo load malsucedida, save e recarga.
- `cargo build -p rocketcon-ffi` gerou a biblioteca estática e o cabeçalho CXX.
- O consumidor C++ da Fase 4 compilou e executou três ticks pelo CXX.

Referências: [guia oficial de build do CXX](https://cxx.rs/build/cargo.html), [tipos Rust opacos e métodos](https://cxx.rs/extern-rust.html), [tratamento de `Result`](https://cxx.rs/binding/result.html).
