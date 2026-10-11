# Rocketcon — circulação planetária compartilhada no vento

Data: 2026-10-07. Continuação do [reuso da temperatura local](rocketcon-phase-1-temperature-reuse.md).

## Mudança

O perfil de vento calcula temperaturas advectivas em duas latitudes vizinhas para estimar o gradiente térmico. Cada cálculo repetia a circulação planetária com o mesmo planeta e a mesma época. Na entrada do vento que recebe a temperatura local pronta, a circulação agora é calculada uma vez e passada às duas temperaturas vizinhas. A entrada antiga e a função pública de temperatura preservam suas assinaturas e calculam a circulação normalmente.

O valor compartilhado existe somente durante a avaliação do vento; não há cache global, persistente ou entre ticks. O benchmark compara o diagnóstico do vento das entradas antiga e nova por igualdade exata e repetiu a avaliação aerodinâmica com resultado idêntico nos cenários de atmosfera e motor ativo.

## Medição

Build release no Intel Core i5-9400F, Windows, `dt = 0,02 s`. Medianas sem tracing SQL, após a primeira das 20 amostras:

| Medida | Antes desta etapa | Depois |
| --- | ---: | ---: |
| Avaliação aerodinâmica isolada, atmosfera | 38,94 ms | 29,54 ms |
| Vento dentro da avaliação, atmosfera | 26,01 ms | 16,61 ms |
| Leituras SQL por avaliação | 479 | 366 |
| Tick completo, atmosfera | 140,33 ms | 106,00 ms |
| Tick completo, motor ativo | 140,16 ms | 105,98 ms |
| Leituras SQL por tick, atmosfera | 1.673 | 1.334 |
| Leituras SQL por tick, motor ativo | 1.694 | 1.355 |

A avaliação deixou de iniciar **113 leituras SQL** (23,6%). Como o tick atmosférico faz três avaliações, deixou de iniciar **339 leituras** (20,3%). As escritas permaneceram 13 por tick atmosférico e 14 por tick com motor ativo. As contagens são de instruções iniciadas, não de linhas lidas. Tempos de execuções separadas variam com a carga do computador; o ganho deve ser acompanhado em novas medições.

## Próxima investigação

As três temperaturas da mesma avaliação ainda calculam separadamente a média global e percorrem planeta, estrela, atmosfera e efemérides. O próximo candidato é um contexto climático explícito por planeta e época, com dados derivados compartilhados entre latitudes. Antes de usá-lo no tick inteiro, medir a igualdade de vento e aerodinâmica em mais de uma latitude e época e comparar a contagem SQL.
