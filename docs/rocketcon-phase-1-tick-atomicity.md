# Rocketcon — atomicidade do tick completo

Data: 2026-10-07. Continuação da [persistência térmica atômica](rocketcon-phase-1-thermal-atomicity.md).

## Resultado

As entradas públicas do tick agora executam todo o cálculo em uma transação SQLite. Um pool privado de uma conexão mantém a mesma sessão entre as chamadas dos repositórios, que continuam recebendo `&SqlitePool`. O tick inicia `BEGIN IMMEDIATE`, confirma apenas depois do relatório final e executa rollback se qualquer etapa retornar erro. Se a operação for abandonada, a guarda agenda rollback e só então libera a sessão para outro tick.

Para vários ticks, `TickTransactionSession` reutiliza a conexão privada e serializa chamadas concorrentes com uma trava assíncrona. A entrada tradicional continua disponível para um tick isolado. O benchmark usa a sessão reutilizável. As funções de clima e persistência térmica reconhecem o contexto de transação já existente e não tentam abrir uma segunda transação na mesma conexão.

`BEGIN IMMEDIATE` impede escritas externas concorrentes enquanto o tick está em andamento. O pacote climático inicial permanece válido até o fim do tick, então a releitura antes da aerodinâmica final foi removida nesse fluxo. O carregamento climático independente mantém seu snapshot curto e a API de atualização para outros usos.

## Testes de atomicidade

`late_tick_failure_rolls_back_every_table` usa um save descartável com motor ativo. Um gatilho provoca falha durante a gravação térmica, depois que bateria e dinâmica já foram calculadas. O teste compara todas as tabelas antes e depois do erro e não encontra alterações. Repete a falha pela API da sessão reutilizável, remove o gatilho e confirma que o tick volta a funcionar. Dois ticks concorrentes no mesmo veículo avançam a época duas vezes, sem perder atualização.

O teste do pool transacional confirma que consultas e gravações feitas em checkouts separados continuam na mesma transação, que rollback e commit têm os resultados esperados, que o abandono da guarda libera a transação e que duas transações da mesma sessão são serializadas. O teste de lote térmico mantém a verificação de rollback entre nós.

`cargo test -p rocketcon-db -p rocketcon-app -p rocketcon-sim --offline` passou. O benchmark release de 100 ticks passou nos cenários coast, atmosfera e motor ativo. Os valores finais de velocidade continuaram em 35656,61 m/s, 4537,22 m/s e 3473,61 m/s. Os últimos diagnósticos aerodinâmicos continuaram nos ticks 45 e 48 para atmosfera e motor ativo.

## Custo observado

Com a sessão reutilizável, uma execução release de 100 ticks mediu medianas de 14,79 ms para coast, 21,21 ms para atmosfera e 29,71 ms para motor ativo, excluindo o primeiro tick. As execuções anteriores à transação completa mediram 13,12 ms, 17,34 ms e 23,49 ms, respectivamente. A carga da máquina introduz variação, mas manter uma conexão privada para cada tick isolado mostrou custo maior: 26,93 ms, 33,62 ms e 41,24 ms na mesma série de 100 ticks. A sessão é o caminho indicado para uma simulação contínua.

O rastreador SQL foi adaptado para a conexão privada. Em 20 ticks, leituras/escritas por tick foram 157/12 em coast, 298/13 em atmosfera e 319/14 com motor ativo. As seis leituras climáticas a menos nos dois últimos cenários vêm da remoção da releitura final, que se tornou redundante dentro da transação do tick. O modo SQL tem custo de instrumentação e não serve para comparar latência.

## Limites

A atomicidade cobre uma chamada de tick. Operações públicas independentes, como persistir uma trajetória ou avançar apenas a rede térmica, mantêm os próprios contratos. A transação segura o bloqueio de escrita do SQLite durante o cálculo inteiro; testes com vários veículos e edições externas simultâneas ainda são necessários para caracterizar contenção em cargas reais. Não houve mudança de contrato com o repositório da interface desktop.
