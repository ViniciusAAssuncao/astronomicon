# Rocketcon — verificação dos registros climáticos antes do estado final

Data: 2026-10-07. Continuação dos [registros planetários compartilhados](rocketcon-phase-1-shared-body-inputs.md).

## Etapa definida

Antes da avaliação aerodinâmica final, o tick relê planeta, estrela parental, atmosfera e hidrosfera quando o veículo permanece no mesmo corpo de referência. Se algum valor diferir do pacote usado no início, o pacote em memória é substituído. Se o corpo de referência mudar, a avaliação final usa o carregamento normal do novo corpo.

Isso permite compartilhar os dados de entrada quando permanecem iguais e evita usar o pacote inicial após uma alteração já confirmada no banco. O cálculo temporal continua separado para as épocas inicial e final.

## Validação

O benchmark altera o albedo planetário na sua cópia descartável do save, verifica que a mudança foi detectada, restaura o valor original e verifica novamente a atualização do pacote. Em seguida, confirma que uma nova leitura sem alteração não sinaliza mudança. As comparações de temperatura, vento e diagnóstico aerodinâmico também passaram nos cenários atmosférico e com motor ativo.

As execuções de 20 e 100 ticks terminaram com estado persistido válido. A saída da atmosfera continuou nos ticks 45 e 48 nos cenários atmosférico e com motor ativo, com os mesmos valores finais de altitude e velocidade da etapa anterior.

## Custo medido

| Cenário | Leituras SQL antes | Leituras SQL depois | Escritas SQL depois |
| --- | ---: | ---: | ---: |
| Coast | 157 | 157 | 12 |
| Atmosfera | 298 | 304 | 13 |
| Motor ativo | 319 | 325 | 14 |

A verificação custa seis leituras SQL por tick nos cenários com atmosfera. Em uma medição release de 20 ticks, excluindo o primeiro, a mediana foi 22,07 ms em atmosfera e 23,21 ms com motor ativo. Os tempos variam com a carga do computador; a contagem SQL é o indicador determinístico desta etapa.

## Limite de consistência

A verificação detecta mudanças confirmadas antes da releitura, mas não é uma transação que abrange o tick inteiro. Uma escrita externa após a releitura, ou entre as consultas que compõem o pacote, ainda pode produzir uma visão mista. O próximo checkpoint é avaliar a unidade transacional do tick e as interfaces dos repositórios para obter uma visão consistente sem bloquear o fluxo de simulação por tempo excessivo.
