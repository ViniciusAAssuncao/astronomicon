# Rocketcon — perfil do tick e contagem SQL

Data: 2026-10-07. Continuação da [primeira medição](rocketcon-phase-1.md). Código em `dev-version0.6-unstable`, Intel Core i5-9400F, Windows, build release, `dt = 0,02 s`.

## Reprodução

```powershell
$env:ROCKETCON_PROFILE='stages'
cargo run --release -p rocketcon-sim --bin flight_tick_bench --offline -- 20
$env:ROCKETCON_PROFILE='sql'
target\release\flight_tick_bench.exe 3
Remove-Item Env:ROCKETCON_PROFILE
```

`stages` mede oito intervalos consecutivos dentro do tick, mantendo o pool habitual. `sql` instala `sqlite3_trace_v2` em uma única conexão, conta instruções SQL realmente iniciadas e mostra as tabelas principais. O callback SQL tem overhead; seus tempos **não** entram na tabela de latências. A preparação do save e o primeiro tick ficam fora das medianas de 20 ticks.

## Tempo por etapa sem tracing SQL

Medianas em ms para os 19 ticks após o primeiro:

| Etapa | Coast | Atmosfera | Motor ativo |
| --- | ---: | ---: | ---: |
| Carregar estado, ambiente e snapshot | 2,68 | 2,96 | 3,23 |
| Potência e bateria | 2,74 | 2,91 | 3,33 |
| Calor gerado por componente | 1,88 | 1,96 | 2,18 |
| Aerodinâmica e contato iniciais | 0,15 | 51,68 | 54,04 |
| Dinâmica ou coast | 1,17 | 57,55 | 60,89 |
| Efemérides, aerodinâmica final e escudo | 0,54 | 52,06 | 53,96 |
| Montar, avançar e persistir rede térmica | 2,11 | 2,19 | 2,45 |
| Gravidade final, Max-Q e relatório | 0,79 | 1,18 | 1,34 |
| **Tick completo** | **12,19** | **173,00** | **185,15** |

Os três intervalos com aerodinâmica respondem por aproximadamente 161 ms em atmosfera e 169 ms com motor ativo, somando as medianas das etapas. Dentro de `dynamics_or_coast`, `advance_vehicle_physical_state` recalcula ambiente, gravidade e aerodinâmica; portanto o custo dessa etapa não é apenas integração RK4. As medianas de etapas não precisam somar exatamente à mediana do tick, pois são distribuições distintas.

## SQL executado por tick

Resultado do modo `sql` em release com três ticks por cenário. O primeiro e os dois seguintes tiveram a mesma contagem em cada cenário:

| Cenário | SELECT/WITH por tick | INSERT/UPDATE/DELETE/REPLACE por tick |
| --- | ---: | ---: |
| Coast | 157 | 12 |
| Atmosfera | 2.132 | 13 |
| Motor ativo | 2.153 | 14 |

No cenário atmosférico, em três ticks, as tabelas mais presentes como fonte principal de instruções foram `planets` (1.326; 442/tick), `stars` (1.266; 422/tick), `barycenters` e `minor_planets` (846 cada; 282/tick), `atmospheres` e `atmosphere_gas_components` (609 cada; 203/tick). Esses números são **instruções iniciadas**, não linhas lidas, e a classificação por tabela usa o primeiro `FROM`/`INTO`/`UPDATE` do texto SQL. Uma consulta com joins conta apenas para uma tabela nesse resumo. Consultas internas do SQLite e custos individuais de cada instrução não são estimados.

## Leitura técnica

`resolve_vehicle_aerodynamics` aparece no tick inicial, dentro da propagação dinâmica e novamente no estado final. Cada chamada percorre funções climáticas que consultam repetidamente planeta, estrela, atmosfera e efemérides. O perfil aponta essa cadeia como alvo inicial de investigação; ele ainda não separa o custo de `resolve_advective_surface_temperature`, `resolve_atmospheric_profile_at_altitude` e `resolve_wind_profile_at_latitude`, nem demonstra quanto do tempo é CPU versus espera do SQLite.

## Avaliação aerodinâmica isolada

O modo `aero-stages` recria o cenário de atmosfera a 10 km e avalia repetidamente o mesmo estado, ambiente, componentes e época, sem avançar a física. A cada repetição, compara o diagnóstico inteiro com a API anterior. O modo `aero-sql` usa uma conexão rastreada e conta as instruções, mas seus tempos incluem overhead do callback.

```powershell
$env:ROCKETCON_PROFILE='aero-stages'
cargo run --release -p rocketcon-sim --bin flight_tick_bench --offline -- 20
$env:ROCKETCON_PROFILE='aero-sql'
target\release\flight_tick_bench.exe 3
Remove-Item Env:ROCKETCON_PROFILE
```

Medianas em ms, excluindo a primeira das 20 avaliações:

| Etapa | Atmosfera | Motor ativo |
| --- | ---: | ---: |
| Carregar atmosfera, planeta e orientação | 0,28 | 0,26 |
| Temperatura de superfície | 12,65 | 12,05 |
| Atmosfera na altitude | 0,23 | 0,22 |
| Perfil de vento | 38,06 | 36,77 |
| Cálculo aerodinâmico local | 0,02 | 0,03 |
| **Avaliação completa** | **51,54** | **49,53** |

Cada avaliação fez **632 leituras SQL e nenhuma escrita** em ambos os cenários, inclusive a primeira. Em três avaliações atmosféricas, as tabelas principais foram `planets` (426 instruções), `stars` (408), `minor_planets` e `barycenters` (270 cada), `atmospheres` e `atmosphere_gas_components` (201 cada). A classificação por tabela tem a mesma limitação descrita acima.

O perfil confirma que temperatura de superfície e vento concentram praticamente todo o tempo da avaliação. O vento recalcula a temperatura advectiva em três latitudes, incluindo a mesma latitude já calculada pela aerodinâmica. `resolve_advective_surface_temperature` consulta por sua vez a circulação planetária e outras funções climáticas. Esse encadeamento explica a multiplicação de leituras, embora a contagem atual não atribua cada instrução a uma subetapa.

Próximo checkpoint: reutilizar os resultados climáticos imutáveis que têm a mesma época e latitude dentro da avaliação, medir novamente tempo e SQL e validar a igualdade do diagnóstico antes de ampliar o cache para o tick inteiro.
