# Vogls Plan

Vogls is a Verilog simulation library. This Python package provides a wrapper
of that library to perform side-channel analysis on designs.

> [!WARNING]
> 
> Although we use Vogls Plan for our own research, it is still very much
> *alpha* software. We hope to develop in the open and collaborate to resolve
> bugs and implement missing features.

Vogls Plan is a declarative API, similar to [SQL] or [Polars], to perform
side-channel analysis. In a declarative or lazy API, you describe a _plan_ for
a computation, meaning that function calls are not immediately evaluated. In
Vogls Plan, your plan is evaluated when you call `.compute()`. At which time,
the plan will be materialized and executed. This way the plan can be smartly
optimized and parallelized i.e. allowing much more efficient evaluation.

# Usage

Below is an example to perform _Test-Vector Leakage Assessment_ (TVLA) that
shows all the basic concepts of this library. You can output a DOT graph of the
computation plan by calling `.to_dot_graph()` where you would call
`.compute()`.

```python
import vogls as vg
from random import randint

CYCLE = 2
N_CYCLES = 8

NUM_RUNS = 5

def run(*, random: bool) -> vg.LazyRunVector:
    # Initialize a design from the `xor.v` file.
    design = vg.LazyDesign("xor.v")

    # Describe a run for the design.
    r = design.run()

    # Either set the input signals or do nothing per run. Since the input
    # arrays are a certain length, that many runs will happen.
    if random:
        r = r.set_signal(
            "a", vg.LazyArray.random_bits(NUM_RUNS, 32, seed=randint(0, 100))
        )
        r = r.set_signal(
            "b", vg.LazyArray.random_bits(NUM_RUNS, 32, seed=randint(0, 100))
        )
    else:
        r = r.repeat(NUM_RUNS)

    r = (
      r
        # Start tracing the signal activity.
        .trace_start()

        # Run the simulation for a certain amount of clock cycles.
        .run_for(CYCLE * N_CYCLES)

        # Get the Hamming distance for each time step since tracing started.
        .hamming_distance("hd")

        # Finish the simulation run.
        .finish()
    )

    # Get both the Hamming distance and the time step value.
    hd = r.get("hd", vg.LazyPlan)
    dist = hd.get("dist", vg.LazyRunVector)
    time = hd.get("time", vg.LazyRunVector)

    # Create one value per clock cycle by all Hamming distance values per cycle.
    return dist.window_sum(by=time, width=CYCLE, start=0, end=CYCLE * N_CYCLES)


# Define two plans.
fixed = run(random=False)
random = run(random=True)

# Compute the TVLA score for each cycle.
print(vg.welch_t_test(fixed, random).compute())
```

This utilizes the following computation plan. As you can see, the design is deduplicated and shared between both runs.

<p align="center">
  <img src="./assets/tvla.svg" width="100%" />
</p>

[SQL]: https://en.wikipedia.org/wiki/SQL
[Polars]: https://en.wikipedia.org/wiki/Polars_(software)
