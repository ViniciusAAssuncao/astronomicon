# Rocketcon — dados de superfície compartilhados por avaliação

Data: 2026-10-07. Continuação do [contexto climático](rocketcon-phase-1-climate-context.md).

## Próximo passo definido e executado

O contexto climático já compartilhava temperatura média global e circulação planetária. Ainda consultava planeta, estrela, atmosfera e efemérides ao calcular cada uma das três latitudes. A avaliação agora carrega uma vez os dados de superfície que dependem apenas do planeta e da época: declinação, irradiância no topo da atmosfera, efeito estufa e albedo. A função antiga de temperatura carrega os mesmos dados por chamada e usa a mesma rotina de cálculo local.

O contexto continua limitado a uma avaliação aerodinâmica. Nenhum resultado é mantido entre ticks ou entre veículos.

## Verificação

O benchmark comparou por igualdade exata as temperaturas antiga e compartilhada na latitude do veículo e nas duas latitudes vizinhas, e também em −60° e +60° em outro instante. O vento antigo e o vento com contexto também coincidiram. As comparações passaram nos cenários de atmosfera e motor ativo. Uma execução de 100 ticks por cenário concluiu com estado persistido válido; a aerodinâmica foi calculada nos primeiros 45 ticks atmosféricos e nos primeiros 48 com motor ativo, antes da saída da atmosfera modelada.

## Medição

Build release, Intel Core i5-9400F, Windows, `dt = 0,02 s`. Medianas sem tracing SQL de 19 amostras após a primeira. A primeira rodada desta etapa teve variação de latência; a tabela usa a repetição mais estável. Leituras SQL foram idênticas por avaliação e por tick nas amostras rastreadas.

| Medida | Antes desta etapa | Depois |
| --- | ---: | ---: |
| Avaliação aerodinâmica isolada, atmosfera | 14,00 ms | 11,55 ms |
| Vento dentro da avaliação, atmosfera | 3,14 ms | 1,04 ms |
| Leituras SQL por avaliação | 169 | 145 |
| Tick completo, atmosfera | 64,27 ms | 51,87 ms |
| Tick completo, motor ativo | 58,71 ms | 52,38 ms |
| Leituras SQL por tick, atmosfera | 743 | 671 |
| Leituras SQL por tick, motor ativo | 764 | 692 |

Cada avaliação deixou de iniciar 24 leituras SQL (14,2%); as três avaliações de um tick atmosférico deixaram de iniciar 72 leituras (9,7%). A contagem de escritas por tick permaneceu 13 em atmosfera e 14 com motor ativo. O tempo de uma execução isolada varia com a carga do sistema, por isso a contagem SQL é o sinal mais estável deste checkpoint.

## Próximo checkpoint

O carregamento do contexto ainda responde pela maior parte do tempo aerodinâmico. Antes de compartilhar esse contexto entre as três avaliações do tick, mapear quais entradas variam entre estado inicial, propagação e estado final. É necessário preservar as épocas e posições próprias de cada avaliação, inclusive a transição para fora da atmosfera.
