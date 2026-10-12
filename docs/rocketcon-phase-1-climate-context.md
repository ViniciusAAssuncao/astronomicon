# Rocketcon — contexto climático por planeta e época

Data: 2026-10-07. Continuação do [reuso da circulação](rocketcon-phase-1-circulation-reuse.md).

## Mudança

`AdvectiveTemperatureContext` carrega uma vez a temperatura média global e a circulação planetária para um planeta, uma época universal e um instante da simulação. A aerodinâmica usa esse contexto para calcular a temperatura local e as temperaturas das duas latitudes vizinhas usadas no perfil de vento. O contexto pertence a uma avaliação aerodinâmica; não é global nem atravessa ticks.

As funções públicas anteriores de temperatura, circulação e vento continuam disponíveis. A implementação comum recebe os resultados prontos quando há contexto e resolve esses valores normalmente nas chamadas antigas.

## Validação

O benchmark comparou por igualdade exata:

- Temperatura advectiva antiga e compartilhada na latitude do veículo e nas duas latitudes vizinhas.
- Temperatura e vento em latitudes de −60° e +60° em outro instante (`at_epoch + 600 s`).
- Vento antigo e vento com contexto na latitude do veículo.
- Diagnóstico aerodinâmico das avaliações repetidas com estado e época fixos.

Essas comparações passaram nos cenários de atmosfera e motor ativo. O tick completo continuou produzindo as mesmas posições, velocidades e altitudes finais mostradas nos benchmarks anteriores de 20 ticks. A comparação cobre as condições medidas; ela não é uma prova para todos os planetas e atmosferas.

## Medição

Build release, Intel Core i5-9400F, Windows, `dt = 0,02 s`. Medianas de 19 amostras após a primeira, sem tracing SQL:

| Medida | Antes desta etapa | Depois |
| --- | ---: | ---: |
| Avaliação aerodinâmica isolada, atmosfera | 29,54 ms | 14,00 ms |
| Perfil de vento dentro dela, atmosfera | 16,61 ms | 3,14 ms |
| Leituras SQL por avaliação | 366 | 169 |
| Tick completo, atmosfera | 106,00 ms | 64,27 ms |
| Tick completo, motor ativo | 105,98 ms | 58,71 ms |
| Leituras SQL por tick, atmosfera | 1.334 | 743 |
| Leituras SQL por tick, motor ativo | 1.355 | 764 |

Cada avaliação deixou de iniciar **197 leituras SQL** (53,8%) e o tick atmosférico, com três avaliações, deixou de iniciar **591 leituras** (44,3%). A contagem de escritas permaneceu 13 por tick atmosférico e 14 com motor ativo. As contagens representam instruções SQL iniciadas, não linhas lidas. Tempos de execuções separadas variam com a carga do computador; os números são uma comparação inicial, não um limite garantido.

## Próxima decisão técnica

O contexto ainda chama as funções atuais para irradiância, atmosfera e dados do planeta ao calcular cada latitude. O custo restante da avaliação fica principalmente no carregamento inicial do contexto. Antes de compartilhar resultados entre avaliações aerodinâmicas do mesmo tick, é necessário definir explicitamente quais entradas podem variar com posição, época e estado e medir o efeito em um cenário mais longo.
