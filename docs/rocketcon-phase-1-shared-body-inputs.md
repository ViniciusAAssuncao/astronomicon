# Rocketcon — registros planetários compartilhados entre épocas do tick

Data: 2026-10-07. Continuação do [mapa de carregamento climático](rocketcon-phase-1-climate-load-map.md).

## Implementação

`ClimateBodyInputs` carrega planeta, estrela parental, atmosfera e hidrosfera uma vez. `AdvectiveTemperatureContext::load_with_body` usa esses registros para calcular um contexto novo para cada época. O tick compartilha o pacote de registros entre as avaliações aerodinâmicas inicial e final, mas calcula separadamente posições dos corpos, irradiância, anomalia orbital, temperatura média, circulação e vento. A função de carregamento independente do contexto continua disponível.

O cálculo por época reúne a irradiância no topo da atmosfera, o albedo dinâmico, a média global, as temperaturas equatorial e polar e a circulação sem reler os mesmos registros em cada subfunção. A época continua sendo passada às funções de efemérides e emissão estelar. O caminho sem atmosfera permanece válido.

## Igualdade e execução longa

O benchmark comparou por igualdade exata temperaturas e vento entre contextos com carregamento independente e com pacote compartilhado, em três latitudes e duas épocas. Também comparou as funções climáticas antigas em latitudes vizinhas e em outro instante. As comparações passaram para Hadab, com atmosfera e hidrosfera, e Meros, sem ambas. O diagnóstico aerodinâmico repetido permaneceu idêntico.

O benchmark de 100 ticks terminou com estado persistido válido. Os cenários atmosférico e com motor ativo deixaram a atmosfera modelada após os ticks 45 e 48, respectivamente, com as mesmas altitudes e velocidades finais medidas antes da alteração.

## SQL e tempo

Build release, Intel Core i5-9400F, Windows, `dt = 0,02 s`. Contagens SQL por chamada, em conexão rastreada:

| Etapa climática | Antes | Depois |
| --- | ---: | ---: |
| Carregar contexto completo | 125 | 14 |
| Registros do corpo, dentro das 14 leituras | — | 6 |
| Cálculos específicos da época, dentro das 14 leituras | — | 8 |
| Temperatura em latitude adicional | 0 | 0 |
| Perfil de vento | 13 | 13 |
| Avaliação aerodinâmica completa | 145 | 34 |

No tick completo, o pacote de registros é carregado uma vez e os contextos das épocas inicial e final são calculados separadamente:

| Medida | Antes desta etapa | Depois |
| --- | ---: | ---: |
| Leituras SQL por tick, atmosfera | 526 | 298 |
| Leituras SQL por tick, motor ativo | 547 | 319 |
| Mediana do tick, atmosfera | 39,96 ms | 22,45 ms |
| Mediana do tick, motor ativo | 39,74 ms | 24,78 ms |

O cenário coast manteve 157 leituras por tick. A contagem de escritas permaneceu 13 em atmosfera e 14 com motor ativo. As contagens SQL são instruções iniciadas, não linhas lidas. As latências são medianas de 19 ticks após o primeiro e variam com a carga do computador.

## Limite do compartilhamento

O fluxo do tick não altera os registros astronômicos, mas uma escrita externa concorrente entre as duas avaliações poderia torná-los diferentes. Esse caso não é detectado pelo pacote em memória. O próximo checkpoint deve considerar uma visão consistente do banco durante o tick ou um mecanismo explícito de revisão dos registros antes de ampliar o reuso para outras partes da simulação.
