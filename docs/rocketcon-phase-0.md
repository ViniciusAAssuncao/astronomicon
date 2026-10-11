# Rocketcon runtime — Fase 0

## Objetivo e estado encontrado

Preparar uma medição do tick completo antes de decidir se o runtime precisa de estado residente em memória. Este documento descreve o fluxo observado no checkout `dev-version0.6-unstable` em 2026-10-07; contagens de SQL e latências ainda não foram medidas.

`rocketcon-sim::run_bridge_smoke_test` carrega um `EnvironmentSnapshot` uma vez e repete somente `gravitational_acceleration_at_snapshot`. O tempo que ele imprime não representa `advance_vehicle_simulation`. Não há save Rocketcon local em `saves/`; `database/astronomicon.db` contém o modelo astronômico, sem tabelas de veículos. `rocketcon_app::build_context` cria/abre um save e aplica as migrações Rocketcon, enquanto `astronomicon_db::connection::open_pool` aplica as migrações Astronomicon.

Baseline executado: `cargo check -p rocketcon-app -p rocketcon-sim --offline` passou. `cargo test -p rocketcon-app -p rocketcon-sim --offline` passou, mas os dois crates atualmente não possuem testes unitários ou de documentação executados. Isso comprova compilação, não correção do tick completo. O primeiro teste comportamental útil depende do fixture da Fase 1.

## Caminho de um tick completo

Entrada: `advance_vehicle_simulation(pool, vehicle_id, dt, universe_epoch, control_input)` em `crates/rocketcon-app/src/aeroespacial/simulation_tick.rs`.

| Etapa | Trabalho observado | Acesso a dados |
| --- | --- | --- |
| Estado inicial | Carrega estado físico, ambiente e `VehicleSnapshot`. | Lê estado físico, planeta, estrela/hierarquia, posições do sistema, atmosfera, montagem e estados dos componentes. O snapshot consulta reservatório e estado operacional por componente; cargas úteis podem causar consultas recursivas. |
| Energia | Calcula geração/consumo e aplica delta nas baterias. | Relê lista de componentes e estados operacionais; geração solar consulta efemérides. Lê e atualiza reservatórios quando presentes. |
| Calor dos componentes | Obtém componentes e contribuições de geração/consumo para cada estágio ativo. | Lê componentes e estados operacionais; alguns tipos consultam reservatório ou efemérides. |
| Aerodinâmica e contato inicial | Resolve atmosfera, vento, temperatura, orientação planetária e contato. | Lê atmosfera, planeta e dados climáticos indiretos. Orientação busca planeta. Sem atmosfera, parte do cálculo climático é pulada. |
| Ramo ativo | Se houver propulsão, arrasto ou contato: integra corpo rígido via RK4. | Relê estado, componentes, snapshot, ambiente, gravidade e aerodinâmica; consulta estados operacionais/rodas. Atualiza gimbal e roda quando aplicável, grava estado físico e invalida patches futuros. |
| Ramo coast | Caso contrário: propaga patch orbital. | Lê estado e patches; pode gerar patches se faltarem/expirarem, com consultas de corpos/SOI e gravações dos patches. Consulta posição/velocidade do corpo e grava estado físico. |
| Estado final | Resolve novas efemérides, contato, aerodinâmica, escudo, rede térmica, gravidade e cargas G. | Relê dados planetários/climáticos e materiais; escudo pode atualizar ablação. Rede térmica lê materiais, atributos e nós; grava cada nó. Max-Q pode causar outro upsert de estado físico. |

As chamadas de efemérides, clima e gravidade descem para `astronomicon-app` e suas consultas SQL. A tabela mostra **categorias e condições**, não uma contagem fixa de consultas: número e tipos de peças, atmosfera, patch existente e ramo escolhido alteram o total. A instrumentação da Fase 1 deverá contar consultas reais por cenário e por tick.

## Fronteiras a preservar

- Rust é a fonte de verdade do estado científico, físico e persistido.
- A UI futura envia comandos e recebe consultas, snapshots e eventos por DTOs próprios da fronteira.
- A UI não calcula física, não acessa SQLx nem tipos internos Rust; Qt fica fora dos crates de domínio.
- Simulação em passo fixo, publicação de snapshots e renderização têm ritmos independentes.
- Nenhuma decisão de cache, checkpoint ou troca de banco é tomada antes das medições.

## Cenário proposto para a Fase 1

1. Criar um **save descartável** a partir do modelo astronômico, aplicar ambas as séries de migrações e registrar a versão do código e o hash do fixture. Não usar o save pessoal do usuário nem medir `build_context` dentro do tick.
2. Inserir por API/repositório ou fixture versionado um veículo mínimo válido com estado físico, referência a planeta existente e componentes suficientes para os caminhos de potência e térmica. Fixar IDs, orientação, posição, velocidade, estágios, estados operacionais e epoch. Evitar depender da ordem dos registros do banco local.
3. Executar cenários separados: **coast sem atmosfera/arrasto**, **voo atmosférico com arrasto** e **propulsão/controle ativo**. Se um cenário não puder ser construído com os modelos atuais, registrar a falha como resultado da Fase 1, sem ajustar a física para fazê-lo passar.
4. Usar `dt` fixo, inicialmente `0,02 s` (50 Hz), com séries curtas e longas. Medir tick completo em build release, depois de aquecimento, em cópias independentes do mesmo save. Registrar mediana, p95, p99, máximo, primeiro tick separado, consultas SQL e gravações por tick, além de hardware e configuração SQLite.
5. Conferir após a série: epoch avançou `N × dt`; posição, velocidade e diagnósticos permanecem finitos; IDs e referência do corpo são válidos; estados persistidos correspondem ao relatório final; uma repetição desde o mesmo fixture produz resultados equivalentes dentro da tolerância numérica definida pelo teste.

O orçamento inicial para 50 Hz é 20 ms por tick, mas isso é apenas referência de comparação, não requisito já aceito. A escolha da frequência e de um possível runtime residente depende dos resultados.

## Próximo checkpoint

Implementar o fixture e o benchmark do tick real sem refatorar o runtime. Separar tempo de preparação, primeiro tick e ticks estabilizados. Registrar o perfil de leituras/gravações antes de propor otimização.
