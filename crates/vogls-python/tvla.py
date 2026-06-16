import vogls as vg
from random import randint
import time

CYCLE = 2
N_CYCLES = 8

NUM_RUNS = 10_000


def run(*, random: bool) -> vg.LazyRunVector:
    design = vg.LazyDesign("xor.v")
    r = design.run()

    if random:
        r = r.set_signal(
            "a", vg.LazyArray.random_bits(NUM_RUNS, 32, seed=randint(0, 100))
        )
        r = r.set_signal(
            "b", vg.LazyArray.random_bits(NUM_RUNS, 32, seed=randint(0, 100))
        )

    r = r.trace_start().run_for(CYCLE * N_CYCLES).hamming_distance("hd").finish()
    hd = r.get("hd", vg.LazyPlan)
    dist = hd.get("dist", vg.LazyRunVector)
    time = hd.get("time", vg.LazyRunVector)
    return dist.window_sum(by=time, width=CYCLE, start=0, end=CYCLE * N_CYCLES)


def masked_run(*, random: bool) -> vg.LazyRunVector:
    design = vg.LazyDesign("masked_xor.v")
    r = design.run()

    r = r.set_signal(
        "a_1", vg.LazyArray.random_bits(NUM_RUNS, 32, seed=randint(0, 100))
    )
    r = r.set_signal(
        "b_1", vg.LazyArray.random_bits(NUM_RUNS, 32, seed=randint(0, 100))
    )

    if random:
        r = r.set_signal(
            "a_0", vg.LazyArray.random_bits(NUM_RUNS, 32, seed=randint(0, 100))
        )

    r = r.trace_start().run_for(CYCLE * N_CYCLES).hamming_distance("hd").finish()
    hd = r.get("hd", vg.LazyPlan)
    dist = hd.get("dist", vg.LazyRunVector)
    time = hd.get("time", vg.LazyRunVector)
    return dist.window_sum(by=time, width=CYCLE, start=0, end=CYCLE * N_CYCLES)


start = time.time()
fixed = run(random=False)
random = run(random=True)
print(f"Time: {time.time() - start}s")

start = time.time()
print(vg.t_test(fixed, random).compute().as_list())
print(f"Time: {time.time() - start}s")
