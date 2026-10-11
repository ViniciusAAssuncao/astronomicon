# Rocketcon runtime — primeira medição do tick completo

Data: 2026-10-07. Checkout: `dev-version0.6-unstable`. Máquina: Intel Core i5-9400F, 6 núcleos/6 threads, Windows. Compilação: `--release --offline`.

## Como reproduzir

Na raiz do repositório:

```powershell
cargo run --release -p rocketcon-sim --bin flight_tick_bench --offline -- 20
cargo run --release -p rocketcon-sim --bin flight_tick_bench --offline -- 100
```

O binário copia `database/astronomicon.db` para `saves/bench-{coast,atmosphere,powered}.db`, aplica as migrações e substitui apenas esses três saves de benchmark a cada execução. `saves/` é ignorado pelo Git. O tempo de preparação é separado do tick. Cada cenário começa com dados idênticos em sua própria cópia. `dt = 0,02 s` e `captured_at_epoch` inicial é `1 s`.

O veículo sintético possui 8 CPUs, uma bateria com reservatório persistido e um painel solar. O cenário powered acrescenta um motor SingleBurn de 50 kN com combustível integral e estado operacional de 50%. Coast usa Meros sem atmosfera e um patch cônico inicial; atmosfera e powered usam Hadab a 10 km, com velocidade inicial inercial calculada pela velocidade do planeta mais 100 m/s relativos. O estado físico persistido é confrontado com o último relatório e a epoch deve avançar exatamente pelo número de ticks.

## Resultado release — janela de 20 ticks

O primeiro tick não entra nos percentis. Os 19 seguintes constituem a amostra. Valores em ms/tick:

| Cenário | Primeiro | Mediana | p95 | Ticks com aerodinâmica | Observação |
| --- | ---: | ---: | ---: | ---: | --- |
| Coast | 34,96 | 12,28 | 14,96 | 0/20 | Patch cônico já persistido. |
| Atmosphere | 180,04 | 179,95 | 191,81 | 20/20 | Arrasto e rede térmica ativos. |
| Powered | 172,02 | 186,55 | 228,32 | 20/20 | Motor operacional e integração de corpo rígido. |

Nenhum tick dessa janela registrou contato com superfície. Preparação dos saves levou 228–283 ms e ficou fora da medição por tick.

## Série de 100 ticks e interpretação

Coast completou 100 ticks: mediana 12,53 ms; p95 20,58 ms; máximo 405,23 ms. Houve um pico isolado; essa amostra não basta para diagnosticar sua causa.

Atmosphere completou 100 ticks, mas só os primeiros 45 retornaram diagnóstico aerodinâmico. Powered também completou, com aerodinâmica em 48 ticks. As medianas globais (42,45 ms e 112,46 ms) misturam regimes físicos diferentes e **não** estimam o custo sustentado de um voo atmosférico. O último diagnóstico ocorreu perto de 27 km de altitude em ambos os casos. A rápida elevação do fixture sintético deve ser investigada separadamente antes de usá-lo para avaliar fidelidade de voo.

## Incompatibilidade encontrada e corrigida

Uma propagação N-body pode produzir uma série de Chebyshev sem empuxo, atualmente representada como `LowThrustPatchData { thrust: 0 }`. O esquema SQL de `vehicle_trajectory_patches` e o construtor de domínio exigiam empuxo estritamente positivo. O primeiro benchmark longo falhou no `CHECK (thrust_n IS NULL OR thrust_n > 0)`. A migração `1025_allow_zero_thrust_trajectory_patches.sql` preserva os dados existentes e aceita zero; o construtor passou a aceitar empuxo não negativo. A série posterior de 100 ticks persistiu e releu esses patches com sucesso.

Inspeção do save após outra série de 100 ticks confirmou 6 patches `low_thrust` com `thrust_n = 0` em `saves/bench-atmosphere.db`. O cenário powered permaneceu no ramo de dinâmica e não criou patches.

## Decisão após a medição

O caminho coast usual ficou perto do orçamento ilustrativo de 20 ms, com picos ainda não explicados. Os ticks atmosféricos ficaram perto de 180 ms neste fixture e excedem amplamente esse orçamento. **Ainda não sabemos qual fração vem de SQL, clima, efemérides, térmica ou integração.** Não há evidência suficiente para trocar persistência por estado residente em memória agora.

O perfil por etapa e a contagem de consultas/gravações estão no documento [rocketcon-phase-1-profile.md](rocketcon-phase-1-profile.md). Este benchmark inicial mede latência de ponta a ponta; use o documento de perfil para atribuição de custo por subsistema antes de escolher cache, checkpoint ou outras mudanças de runtime.
