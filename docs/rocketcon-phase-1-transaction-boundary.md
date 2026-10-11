# Rocketcon — limite transacional do tick

Data: 2026-10-07. Continuação da [verificação dos registros climáticos](rocketcon-phase-1-body-refresh.md).

## Decisão deste checkpoint

Não envolver o tick atual em uma transação SQLite antes de adaptar as interfaces de acesso ao banco. O tick recebe `&SqlitePool`, e suas funções de leitura e escrita também recebem o pool. Os helpers de consulta executam cada instrução com `fetch_optional(pool)` ou `fetch_all(pool)`. Abrir uma transação em uma conexão separada não incluiria essas instruções; elas continuariam em outras conexões. Um `BEGIN IMMEDIATE` poderia ainda disputar com as próprias escritas do tick. Com pool de uma conexão, reter a conexão e chamar uma função que busca outra no mesmo pool pode impedir o progresso.

O banco opera em modo WAL, com espera por bloqueio configurada em cinco segundos. Isso permite separar leitores e escritor em muitos casos, mas não torna as várias consultas atuais uma única visão. O pacote `ClimateBodyInputs` lê planeta, hierarquia da estrela, atmosfera e hidrosfera em consultas distintas. A busca da estrela pode atravessar planetas, planetas menores e baricentros; a quantidade de consultas depende da hierarquia.

## Limite observado

O tick lê estado físico, ambiente e veículo, grava alterações de bateria, consulta componentes e estado operacional, calcula e persiste a dinâmica ou a trajetória, consulta efemérides e clima final, pode gravar escudo térmico, atualiza a rede térmica e pode gravar pressão dinâmica máxima. A primeira escrita ocorre antes de todas as leituras necessárias para concluir o tick.

O benchmark da etapa anterior contou 304 leituras e 13 escritas SQL por tick atmosférico, e 325 leituras e 14 escritas com motor ativo. Esses números mostram que uma transação de escrita que cubra todo o cálculo permaneceria aberta durante trabalho considerável. Além da contenção entre veículos, erros após as primeiras gravações hoje podem deixar um tick parcialmente aplicado; o escopo de atomicidade exige uma decisão explícita antes de mudar essa semântica.

## Caminho de implementação

1. Tornar as consultas de planeta, estrela, atmosfera, hidrosfera e travessia hierárquica capazes de usar a mesma conexão ou transação, preservando as APIs públicas que recebem `&SqlitePool`. Reutilizar as conversões de registros existentes e manter cada arquivo de origem abaixo de 500 linhas.
2. Carregar `ClimateBodyInputs` dentro de uma transação curta de leitura, incluindo a verificação final. Assim cada pacote corresponde a uma única visão confirmada do banco; pacotes inicial e final ainda podem corresponder a instantes diferentes. Encerrar a transação antes da física e das escritas do tick.
3. Criar um teste com duas conexões: confirmar uma alteração concorrente entre as consultas do pacote e verificar que o resultado contém integralmente a versão anterior ou a nova, nunca uma mistura. Verificar também a travessia por baricentro.
4. Só então considerar a atomicidade de escrita do tick inteiro. Para isso, propagar uma unidade de trabalho única por energia, dinâmica, trajetória e térmica, definir o ponto de leitura e a política de conflito, e testar rollback após uma falha tardia. Medir o tempo da transação e a contenção com ticks concorrentes antes de adotá-la.

## Etapa seguinte

O carregamento consistente e curto de `ClimateBodyInputs` foi implementado e verificado em [snapshot climático](rocketcon-phase-1-climate-snapshot.md). A releitura antes da aerodinâmica final continua necessária para captar mudanças ocorridas durante o cálculo do tick.
