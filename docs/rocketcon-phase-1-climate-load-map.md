# Rocketcon — mapa do carregamento climático restante

Data: 2026-10-07. Continuação do [reuso da avaliação inicial](rocketcon-phase-1-initial-aero-reuse.md).

## Medição por etapa

O benchmark `ROCKETCON_PROFILE=aero-sql` agora conta separadamente as leituras de `AdvectiveTemperatureContext::load`, de `temperature_at_latitude` e do perfil de vento com contexto. Foram usadas as mesmas condições atmosféricas de 10 km, em build release, com a conexão SQLite rastreada.

| Etapa | Leituras SQL por chamada |
| --- | ---: |
| Carregar contexto climático | 125 |
| Temperatura em uma latitude após carregar | 0 |
| Perfil de vento após carregar | 13 |
| Demais passos da avaliação aerodinâmica | 7 |
| **Avaliação aerodinâmica completa** | **145** |

As tabelas mais presentes nas 125 leituras do carregamento foram `planets` (28), `stars` (27), `barycenters` e `minor_planets` (18 cada), `atmospheres` e `atmosphere_gas_components` (13 cada), `hydrospheres` e `hydrosphere_components` (4 cada). Esse resumo usa a primeira tabela encontrada no texto SQL; os números são instruções iniciadas, não linhas lidas. A soma das tabelas pode diferir do total de leituras porque a classificação é heurística.

O tick atmosférico já faz somente duas avaliações aerodinâmicas, uma no estado inicial e outra no final. Cada uma carrega um contexto com sua própria época. A maior parte do custo remanescente está portanto na construção dos contextos, não no cálculo por latitude.

## Dependências entre avaliações

| Dados | Uso entre início e fim do tick |
| --- | --- |
| Registros de planeta, estrela, atmosfera, gases, hidrosfera e hierarquia | Podem ser reutilizados como dados de entrada se permanecerem inalterados durante o tick. |
| Posições dos corpos, anomalia orbital e orientação | Devem ser resolvidas na época de cada avaliação. |
| Irradiância e emissão estelar | Devem ser calculadas para a época de cada avaliação; emissão de buraco negro também consulta estado dependente do tempo. |
| Temperatura média, circulação, declinação e vento | Devem ser recalculados para a época e, quando aplicável, a latitude de cada avaliação. |
| Latitude, altitude e velocidade relativa do veículo | Devem usar o estado físico inicial ou final correspondente. |

Compartilhar o contexto climático completo entre início e fim alteraria entradas dependentes da época. A divisão segura a investigar é entre registros de entrada carregados do banco e cálculos derivados por época. O fluxo normal do tick não modifica os registros astronômicos; uma futura implementação precisa deixar explícito como lida com uma alteração externa concorrente.

## Próximo checkpoint

Criar um pacote de dados de entrada do planeta e do sistema, usado para construir contextos separados nas épocas inicial e final. Comparar temperatura, vento, diagnóstico aerodinâmico e estado final com o caminho atual em mais de uma latitude e em épocas diferentes. Medir novamente as leituras SQL do contexto e do tick antes de ampliar o escopo.
