# C11 — Integridade do solver universal de dois corpos

**Status: APROVADO e ENCERRADO em 12/10/2026, por concordância do autor e da liderança técnica (ChatGPT, tech lead colaborador).** O Marco C permanece aberto. C12–C18 são etapas independentes que exigem autorização específica. A aprovação registra sucesso nos requisitos do C11, não perfeição matemática, cobertura de todos os valores de `f64` nem garantia de exatidão em cenários não exercitados.

Plano diretor: `D:\projects\rocketcon-desktop\docs\marco-c-roadmap-orbital.md`.
Implementação: `crates/rocketcon-core/src/math/orbital/universal.rs` e `universal/{dd,critical_ellipse}.rs`.
Regressões duráveis: `crates/rocketcon-core/tests/{universal_c11,universal_c11_adversarial,universal_c11_high_precision}.rs`.
Sonda reusável de saves: `crates/rocketcon-sim/src/bin/c11_save_probe.rs`.

## 1. Resultado de engenharia e aceite

O algoritmo anterior de raiz universal foi substituído por um híbrido `f64` com avaliações Stumpff estáveis nas regiões tratadas, Newton rápido quando viável, busca por intervalo protegido, orçamento de iterações e erros explícitos nas falhas identificadas. O propagador mantém as APIs Rust de anomalia `chi` e estado cartesiano; preserva as conicas circular, elíptica, quase parabólica e hiperbólica, tempos positivos/negativos e múltiplas revoluções nos corpora validados. O contrato Rust–CXX–Qt, o esquema SQL e o modelo de forças não foram alterados.

Um fallback privado em double-double (DD), **não global**, resolve e reconstrói o estado no primeiro arco de elipses quase radiais próximas de `E=π`; mantém a raiz em DD até calcular as posições e velocidades 3D finais, arredondadas então para `f64`. Para tempo negativo, usa simetria de reversão de velocidade e tempo. O caminho comum permanece `f64` por razões de desempenho.

O autor confirmou compilação local sem erros ou warnings e inspeção Qt sem regressões visuais perceptíveis. No save C10 outbound, a transição SOI ocorreu em ~2,5 s simulados, sem congelamentos ou artefatos e com telemetria aparentemente íntegra. As sondas automáticas do Qt recompilado em Release passaram nas escalas 100% e 150%, tanto em teste sintético quanto em voos reais C9/C10. A avaliação da liderança técnica é favorável ao encerramento do escopo do C11 **com as ressalvas expressas neste relatório**.

## 2. Regiões numéricas críticas

A equação universal de Kepler tem derivada `F'(χ)=r`, o raio instantâneo. Quando `r` se aproxima de zero, resíduo global pequeno não implica raiz confiável: a equação fica mal condicionada. A reconstrução de Lagrange `f,g,fdot,gdot` também pode amplificar cancelamentos numéricos. O fallback DD foi introduzido para um subconjunto desses problemas, não para todo o espaço orbital.

A RC1 seleciona a contingência apenas com entradas finitas e `mu>0`, `r0>0`, energia elíptica `alpha=2/r0-v²/mu>0`, razão transversal `q=(r0/mu)*(v²-vr²)<=1e-4` e anomalia média próxima de `pi+2b`, com `|M-(pi+2b)|<=0.01` e `b=(r·v)*sqrt(alpha)/sqrt(mu)`. O método especial ainda exige momento angular não nulo, primeiro arco elíptico e `|pi-E|<=0.25` rad. Fora do domínio especial, o solver universal geral permanece responsável pela solução ou falha.

**Risco importante:** esses limiares são heurísticos e não há certificação matemática de falsos negativos ou de erro máximo global. Um encontro quase radial fora do predicado pode retornar um estado finito porém impreciso. Em incidentes de telemetria degenerada, velocidade explosiva, aparente salto orbital ou divergência após longo horizonte, investigar condicionamento `F'=r`, ativação do fallback e perda de precisão de `f/g` antes de supor problema no Qt ou SQL.

### Hipóteses e regiões que não estão certificadas

1. **Queda radial e singularidade pontual:** `r→0`, `h→0`, passagens pelo centro e raios submétricos podem perder representabilidade, mesmo que o propagador retorne valores finitos. Colisão, superfície, aerodinâmica e relatividade são outros domínios; não misturar essas decisões ao solver de dois corpos.
2. **Outras fases elípticas:** DD não cobre todos os arcos, `|pi-E|>0.25`, múltiplas revoluções ou qualquer tempo admissível. Fases recuperadas de `χ` muito grande podem perder dígitos significativos e causar cancelamento trigonométrico ou erro na reconstrução.
3. **Hipérboles extremas e longos horizontes:** argumentos negativos grandes de Stumpff usam `sinh` e estão sujeitos a overflow; `sqrt(mu)*dt`, produtos e sementes também podem transbordar. A detecção explícita de falhas conhecidas não constitui prova de detecção de todo erro silencioso.
4. **Vizinhança parabólica:** `alpha≈0` vem de subtrações entre energias de grandeza semelhante. Sinal e magnitude dependem de arredondamento `f64`; a matriz próxima da parábola foi amostrada, não exaustiva.
5. **Faixa completa de ponto flutuante:** subnormais, grande disparidade de componentes, `mu` e `dt` extremos, produtos escalares com cancelamento, `sqrt/div` DD muito além dos valores típicos e representação de vetores 3D não possuem garantia formal.
6. **Resultados plausíveis e classificação:** a busca `f64` controla resíduo e correção estimada, mas não prova intervalar; um `Ok` fora do domínio DD ainda pode esconder imprecisão em órbitas mal condicionadas.

7. **Acúmulo temporal:** ida/volta e conservação passaram em amostras; milhões de passos, trocas repetidas de referência, tempo arbitrário e erro secular não têm garantia global.
8. **Eventos SOI:** validamos saída após ~2,5 s; detecção sub-tick, passos grandes, tangências, cruzamentos múltiplos e hierarquias lunares são C14.
9. **Forças e previsões:** cônica dois-corpos não equivale a trajetória futura sob N-body, arrasto ou propulsão; comparar coast e integração dinâmica em C12, previsões avançadas em C15.
10. **Performance extrema:** não houve A/B pareado do tick com/sem RC1 no mesmo cenário, sessões de horas ou certificação de p99 em outros hardwares.

Esses itens são riscos documentados, não bugs comprovados. Em incidente novo, registrar `r,v,mu,dt`, época, referencial, `chi`, `alpha`, derivada/resíduo, energia e momento angular; consultar referência independente de alta precisão e a decisão do seletor. Não anunciar perfeição nem inventar física para mascarar erro.
## 3. Evidências quantitativas e limites das provas

O caso matemático mais degenerado estudado teve `r0=7e6 m`, `mu=3.986004418e14 m³/s²`, `vt/vc=1e-5`, `vr=0`, `dt=1030.3459097688751 s`, raio final ~0,00036754 m e velocidade ~1,47e9 m/s: caso deliberadamente **fora do domínio físico de navegação Newtoniana**. Antes da contingência, o erro relativo chegou a ~0,429 em posição e ~0,224 em velocidade. Corrigir apenas a raiz universal sem a reconstrução cartesiana em DD não solucionava o problema.

Antes desta limpeza, scripts independentes Python/mpmath de 100 dígitos testaram 225 estados 3D de cinco orientações, cinco razões transversais, três velocidades radiais e três épocas perto de `E=pi`. Após RC1, o maior erro relativo observado da produção foi `5.318146163e-11` (posição) e `2.659344576e-11` (velocidade): zero dos 225 ultrapassou `1e-8`. Antes da RC1 eram dez casos acima de `1e-8`. Também foram investigados 75 casos quase radiais 2D. **Todos são resultados amostrais, não limites garantidos.**

As regressões exercitaram 1.800 estados adversariais com reversão/invariantes, 126 casos próximos da parábola, 216 combinações de escala, casos circulares, hiperbólicos de longo horizonte e referências cartesianas fixas. As raízes comuns mediram em geral centenas de nanossegundos; o caminho DD experimental requereu aproximadamente 14–17 microssegundos por chamada e é deliberadamente raro.

Perfil Release de 150 ticks por save, incluindo custos de sessão e persistência:

| Save | p50 | p95 | p99 | máximo |
| --- | ---: | ---: | ---: | ---: |
| C9 inbound | 24,404 ms | 29,739 ms | 114,931 ms | 460,202 ms |
| C10 outbound | 23,684 ms | 28,979 ms | 58,197 ms | 60,940 ms |
| C10 inbound | 24,000 ms | 25,485 ms | 32,585 ms | 90,871 ms |

Picos não podem ser atribuídos somente ao Kepler sem comparação A/B pareada. O orçamento de 20 ms/tick não está demonstrado; C13 investigará o runtime.

## 4. Aceite e regressões

O autor aprovou o C11 em 12/10/2026. A avaliação técnica concorda com o encerramento do escopo, respeitando os limites acima.
Sondas Release em cópias descartáveis: C9 inbound e stable (60 ticks), C10 outbound e inbound (150 ticks). O outbound cruzou SOI no tick 125; o inbound não cruzou dentro dos três segundos observados. Os previews renderizaram 99/73 pontos (C9) e 2/101 (C10).

A bateria original passou todas as suítes de rocketcon-core em Debug e Release (35 testes ativos, nove experimentos/benchmarks ignorados em Release). Também passaram rocketcon-sim --lib 20/20 e rocketcon-ffi 3/3. A interface Qt Release recompilada passou sondas sintéticas e reais em escalas 100% e 150%, com relatórios em D:\projects\rocketcon-desktop\build-msvc\c11-rc1-reports.

O autor validou compilação sem warnings e sem erros, continuidade visual da aplicação, transição SOI em ~2,5 s sem congelamento ou artefatos e integridade aparente da telemetria. Estes são os comportamentos manualmente aprovados; não certificam casos arbitrários.

## 5. Regressão mínima e limpeza pós-aceite

Mantidas três suítes Rust em crates/rocketcon-core/tests: `universal_c11.rs` (regimes, Stumpff, Lambert, invariantes, reescala, horizontes longos), `universal_c11_adversarial.rs` (1.800 estados e 126 quase parabólicos), e `universal_c11_high_precision.rs` (referências cartesianas e fallback crítico). A sonda `crates/rocketcon-sim/src/bin/c11_save_probe.rs` permanece para provas integradas em cópias de saves.
Scripts Python, protótipos Rust temporários e benchmarks redundantes foram removidos por decisão expressa do autor. A antiga matriz Python/mpmath de 225 estados deixa de ser diretamente executável; seus resultados são evidência histórica, não substituem a criação de nova referência independente caso apareça um defeito.

Regressão: `cargo test -p rocketcon-core --offline`, `cargo test -p rocketcon-core --release --offline`, `cargo test -p rocketcon-sim --lib --offline`, `cargo test -p rocketcon-ffi --offline`, `cargo check -p rocketcon-sim -p rocketcon-ffi --offline`.

## 6. Referências e decisão

Wisdom e Hernandez, *A fast and accurate universal Kepler solver without Stumpff series* (2015), https://doi.org/10.1093/mnras/stv1862. Tommasini e Olivieri, *Two fast and accurate routines for solving the elliptic Kepler equation* (2022), https://doi.org/10.1051/0004-6361/202141423.

**Aprovação mútua:** o autor e a liderança técnica concordam com o encerramento do C11 pelo alcance dos objetivos de integridade e desempenho sem regressão funcional observada. As ressalvas deste registro devem ser consideradas em qualquer problema futuro. Marco C permanece aberto; C12–C18 continuam separados e dependem de autorização. Nenhum commit ou push autorizado.

**Norma permanente de código:** comentários de linha, bloco ou documentação em arquivos de código-fonte são proibidos e impedem o aceite. Justificativas e limitações devem estar em Markdown. A regra está nos `AGENTS.md` locais de ambos os repositórios, atualmente excluídos do versionamento pelas respectivas regras `.gitignore`; este relatório rastreável também registra a decisão.
