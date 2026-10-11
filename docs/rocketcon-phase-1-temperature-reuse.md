# Rocketcon — reuso da temperatura local na aerodinâmica

Data: 2026-10-07. Continuação do [perfil da Fase 1](rocketcon-phase-1-profile.md).

## Mudança

`resolve_vehicle_aerodynamics` já calculava a temperatura advectiva na latitude do veículo para obter a atmosfera na altitude. `resolve_wind_profile_at_latitude` calculava essa mesma temperatura outra vez, além das temperaturas nas latitudes vizinhas para o gradiente. A nova função `resolve_wind_profile_at_latitude_with_temperature` recebe a temperatura local pronta; a entrada antiga continua disponível e usa a mesma implementação interna. O estado e a época não são guardados em cache entre chamadas.

O benchmark calcula o vento pelas duas entradas com o mesmo planeta, latitude e época e exige igualdade exata do diagnóstico. Também compara o diagnóstico aerodinâmico completo em todas as avaliações repetidas do estado fixo. As duas verificações passaram em atmosfera e motor ativo.

## Resultado

Build release no mesmo Intel Core i5-9400F, Windows, `dt = 0,02 s`. Latências são medianas de 19 amostras após a primeira, sem tracing SQL; os números anteriores estão no perfil da Fase 1.

| Medida | Antes | Depois |
| --- | ---: | ---: |
| Avaliação aerodinâmica isolada, atmosfera | 51,54 ms | 38,94 ms |
| Perfil de vento dentro dela, atmosfera | 38,06 ms | 26,01 ms |
| Leitura SQL por avaliação | 632 | 479 |
| Tick completo, atmosfera | 173,00 ms | 140,33 ms |
| Tick completo, motor ativo | 185,15 ms | 140,16 ms |
| Leitura SQL por tick, atmosfera | 2.132 | 1.673 |
| Leitura SQL por tick, motor ativo | 2.153 | 1.694 |

Cada avaliação deixou de iniciar **153 leituras SQL** (24,2%). O tick atmosférico contém três avaliações e deixou de iniciar **459 leituras** (21,5%). A contagem de escritas do tick permaneceu 13 no cenário atmosférico e 14 com motor ativo. As contagens SQL foram iguais no primeiro tick e nos dois seguintes; são instruções iniciadas, não linhas lidas. As medianas de tempo variam com carga do sistema, portanto representam uma medição inicial do efeito, não uma garantia de desempenho.

## Reprodução

```powershell
$env:ROCKETCON_PROFILE='aero-stages'
cargo run --release -p rocketcon-sim --bin flight_tick_bench --offline -- 20
$env:ROCKETCON_PROFILE='aero-sql'
target\release\flight_tick_bench.exe 3
$env:ROCKETCON_PROFILE='stages'
target\release\flight_tick_bench.exe 20
$env:ROCKETCON_PROFILE='sql'
target\release\flight_tick_bench.exe 3
Remove-Item Env:ROCKETCON_PROFILE
```

Próximo checkpoint: investigar as consultas repetidas a planeta, estrela, atmosfera e hierarquia dentro das temperaturas em latitudes vizinhas. Qualquer reuso adicional deve ser limitado por planeta e época, com comparação de diagnóstico e contagem SQL antes de ampliar o escopo ao tick inteiro.
