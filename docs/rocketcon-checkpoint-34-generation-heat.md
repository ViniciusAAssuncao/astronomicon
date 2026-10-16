# Astronomicon — CP34: reutilização do calor de geração

Data: 2026-10-15

## Escopo

O estágio de orçamento elétrico calcula `ComponentPowerContribution` por componente ativo. O estágio posterior de entradas térmicas calculava novamente a geração de todos os componentes, mesmo quando as propriedades relevantes ainda eram idênticas.

O CP34 passa o calor de geração já calculado para a etapa térmica apenas para os componentes `SolarPanel` e `Rtg`. A função anterior de orçamento elétrico permanece disponível e devolve o mesmo contrato de dois resultados. O tick usa a variante interna que inclui um mapa de calor elegível para reutilização.

## Invariantes

- A reutilização ocorre dentro de um único tick, nunca entre epochs ou ticks.
- O mapa só é disponibilizado à etapa térmica quando posição e orientação do snapshot do orçamento coincidem exatamente com o estado físico atual.
- O mesmo ambiente, epoch universal, epoch local e estados operacionais são usados nos dois cálculos.
- A etapa `apply_power_delta_with_components` é executada na posição original e altera reservatórios de bateria. `NuclearReactor` não utiliza o mapa de calor reutilizável; sua geração e calor são resolvidos de novo após as alterações intermediárias.
- Componentes sem calor reutilizável, estágios inativos e divergências de pose preservam o comportamento anterior.
- As fórmulas de calor solar, RTG, consumo elétrico, dinâmica, DOP853 e persistência não foram alteradas.

## Medições Release

Cenários `flight_tick_bench`, 18 ticks, `ROCKETCON_PROFILE=sql`:

| Cenário | Leituras antes/tick | Depois/tick | Escritas/tick | Hash de estado |
| --- | ---: | ---: | ---: | --- |
| `powered-vacuum` | 45 | 41 | 16 | Idêntico |
| `atmosphere` | 74 | 70 | 13 | Idêntico |
| `powered` | 78 | 74 | 16 | Idêntico |

Os cinco grupos persistidos comparados foram `vehicle_physical_states`, `component_operational_states`, `energy_reservoir_states`, `thermal_node_states` e `component_reaction_wheel_states`.

A mediana do estágio `component_heat_inputs` caiu de aproximadamente 0,5 ms para valores entre 0,00 e 0,01 ms nas execuções medidas. Essa medição é específica do componente de calor, não da UI.

Em `atmosphere`, uma execução Release sem perfilamento (40 ticks) registrou p95 de 47,55 ms e máximo de 65,19 ms. Na repetição, mediana 9,58 ms, p95 10,68 ms e máximo 10,70 ms. A variabilidade impede concluir estabilidade de frame pacing do Desktop.

## Validação

O teste completo de `flight_tick_bench` passou (oito testes); as três execuções Release preservaram byte a byte os fingerprints históricos. Os novos resultados não comprovam cenário com RTG ou reator ativo, e o ramo seguro de reatores foi mantido deliberadamente independente.

Não houve alteração em FFI, Desktop, DOP853, trajetórias ou saves canônicos. Nenhum commit ou push foi efetuado.
