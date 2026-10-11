# Rocketcon — reuso da avaliação aerodinâmica inicial no tick

Data: 2026-10-07. Continuação dos [dados de superfície compartilhados](rocketcon-phase-1-surface-inputs.md).

## Mapa das avaliações no tick

| Avaliação | Estado do veículo | Posição do planeta | Época |
| --- | --- | --- | --- |
| Antes da decisão entre dinâmica e coast | Estado persistido no início | Ambiente carregado no início | `current_at_epoch` |
| Dentro da propagação dinâmica | Mesmo estado persistido, relido | Ambiente recarregado para o mesmo instante | `current_at_epoch` |
| Após propagação ou coast | Novo estado calculado | Efemérides resolvidas para o fim | `current_at_epoch + dt` |

As duas primeiras avaliações recebem também o mesmo planeta, componentes e estágios ativos no fluxo normal. A terceira precisa ser refeita: mudam a época, a posição e a velocidade do veículo e, em geral, a posição do planeta. O caminho coast não entra na propagação dinâmica e já evitava a segunda avaliação.

## Implementação

O tick passa o diagnóstico inicial à propagação dinâmica. A propagação só o aceita se estado físico completo, posição do planeta, componentes, estágios ativos e época universal forem iguais aos valores relidos. Quando qualquer entrada diverge, a função aerodinâmica é executada normalmente. A API pública de propagação individual continua disponível e calcula seu próprio diagnóstico.

O reuso pressupõe que os dados planetários da simulação não sejam alterados externamente entre as duas avaliações no mesmo tick. O fluxo medido não os modifica. O teste unitário da guarda cobre mudanças de estado, posição do planeta, estágios e época; a igualdade dos componentes é verificada pela mesma guarda.

## Medição e validação

Build release, Intel Core i5-9400F, Windows, `dt = 0,02 s`. Medianas sem tracing SQL de 19 ticks após o primeiro:

| Medida | Antes desta etapa | Depois |
| --- | ---: | ---: |
| Leituras SQL por tick, atmosfera | 671 | 526 |
| Leituras SQL por tick, motor ativo | 692 | 547 |
| Trecho de propagação, atmosfera | 17,29 ms | 5,78 ms |
| Tick completo, atmosfera | 51,87 ms | 39,96 ms |
| Tick completo, motor ativo | 52,38 ms | 39,74 ms |

A redução de **145 leituras SQL por tick** coincide com o custo medido de uma avaliação aerodinâmica. A contagem de escritas permaneceu 13 no cenário atmosférico e 14 com motor ativo. Em coast, as 157 leituras e 12 escritas por tick permaneceram iguais. As posições, velocidades e altitudes finais dos benchmarks de 20 ticks coincidiram com a etapa anterior. Em 100 ticks, os cenários de atmosfera e motor ativo continuaram saindo da atmosfera nos ticks 45 e 48, respectivamente, com estado persistido válido. A latência apresenta variação entre execuções; a contagem SQL é o sinal mais estável.

## Próximo checkpoint

Restam as avaliações aerodinâmicas nos estados inicial e final, em épocas distintas. O próximo passo é medir o custo de carregar o contexto climático em cada uma e decidir quais dados planetários podem ser reutilizados entre épocas próximas sem alterar os valores físicos. Nenhuma interpolação ou aproximação temporal deve ser introduzida antes de uma validação numérica explícita.
