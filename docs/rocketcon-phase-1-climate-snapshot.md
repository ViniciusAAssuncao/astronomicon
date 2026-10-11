# Rocketcon — snapshot dos dados climáticos

Data: 2026-10-07. Continuação da [avaliação transacional](rocketcon-phase-1-transaction-boundary.md).

## Mudança

`ClimateBodyInputs::load` abre uma transação SQLite de leitura, consulta planeta, estrela parental, atmosfera e hidrosfera na mesma conexão e encerra a transação antes de retornar. A busca da estrela usa a mesma conexão em todos os caminhos, inclusive quando atravessa outro planeta, planeta menor ou baricentro. As consultas reutilizam os SQL e as conversões dos repositórios atuais; as APIs por pool continuam disponíveis.

A transação é curta: não cobre física, energia, dinâmica, térmica nem gravações do tick. A verificação final de mudanças climáticas abre outro snapshot. Portanto os pacotes inicial e final podem representar versões confirmadas diferentes, como necessário para observar alterações durante o tick, mas cada pacote individual vem de uma única versão confirmada.

## Verificação

O teste `climate_body_reads_one_committed_snapshot` usa duas conexões. Depois da primeira leitura, confirma alterações simultâneas em planeta e atmosfera pela segunda conexão. A conexão leitora ainda obtém os valores antigos, e um novo carregamento após o encerramento da transação obtém os valores alterados. O mesmo teste cria um baricentro no save descartável e compara a estrela parental selecionada pela conexão transacional e pela API por pool.

`cargo test -p astronomicon-app -p rocketcon-app -p rocketcon-sim --offline` passou. O benchmark release de 20 e 100 ticks passou nos três cenários. No ensaio de 100 ticks, a avaliação aerodinâmica terminou nos ticks 45 e 48 para atmosfera e motor ativo, respectivamente, com os mesmos valores finais de velocidade e altitude observados antes desta etapa.

## Custo observado

As leituras e escritas SQL por tick permaneceram em 157/12 para coast, 304/13 para atmosfera e 325/14 para motor ativo. Nos cenários atmosféricos, dois carregamentos climáticos por tick acrescentaram quatro instruções de controle transacional: início e encerramento de cada snapshot. Em uma execução release de 20 ticks sem rastreamento SQL, as medianas, excluindo o primeiro tick, foram 11,77 ms, 20,88 ms e 23,64 ms, respectivamente. A medição de tempo varia com a carga do computador.

## Próximo limite

O tick ainda não é atômico como um todo. Uma escrita externa entre os snapshots inicial e final é visível no segundo; uma escrita posterior não é. Gravações de energia e dinâmica podem ser persistidas antes de uma falha tardia. Qualquer transação de escrita do tick exigirá que as funções de persistência compartilhem uma unidade de trabalho e que se meçam a contenção e o rollback antes de adotá-la.
