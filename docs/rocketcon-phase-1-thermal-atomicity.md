# Rocketcon — persistência atômica da rede térmica

Data: 2026-10-07. Continuação do [snapshot climático](rocketcon-phase-1-climate-snapshot.md).

## Etapa definida

A integração térmica calcula a nova rede em memória, consulta os materiais, verifica limites estruturais e monta o diagnóstico antes de gravar os estados dos nós. Todos os estados são construídos e validados antes da primeira gravação. O repositório grava o lote em uma transação SQLite curta e confirma somente quando todos os nós foram atualizados.

Antes desta etapa, a rotina gravava cada nó antes da verificação de materiais. Uma falha nessa verificação podia deixar temperaturas novas persistidas mesmo que o tick retornasse erro. Uma falha de banco no meio do lote também podia deixar alguns nós atualizados e outros antigos.

## Verificação

O teste `batch_rolls_back_all_nodes_when_one_write_fails` cria dois nós e provoca uma falha de restrição na segunda gravação. Confirma que ambos mantêm os valores anteriores; depois confirma que um lote válido atualiza ambos. `cargo test -p rocketcon-db -p rocketcon-app --offline` passou.

O benchmark release de 100 ticks passou nos cenários coast, atmosfera e motor ativo. A saída da atmosfera permaneceu nos ticks 45 e 48 para os dois últimos cenários. As velocidades finais foram 35656,61 m/s, 4537,22 m/s e 3473,61 m/s, iguais às da etapa anterior.

No rastreamento de 20 ticks, leituras e escritas SQL por tick permaneceram em 157/12, 304/13 e 325/14. A transação térmica acrescentou duas instruções de controle por tick. A contagem de instruções de controle por tick passou a 2 no coast e a 6 nos cenários com atmosfera, que já tinham dois snapshots climáticos. A duração oscila com a carga do computador; o resultado físico e a contagem de consultas são as comparações principais.

## Continuação

Esta transação local cobre os nós térmicos para chamadas independentes. O [tick completo agora tem uma transação própria](rocketcon-phase-1-tick-atomicity.md), que inclui bateria, atuadores, estado físico, trajetória, escudo e rede térmica em uma única confirmação. Não foi necessário criar uma transação separada para o conjunto de baterias dentro do tick.
