import vogls as vg
from random import randint
import time

CYCLE = 2
N_CYCLES = 8

NUM_RUNS = 5

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
    else:
        r = r.repeat(NUM_RUNS)

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


fixed = run(random=False)
random = run(random=True)

print(vg.welch_t_test(fixed, random).to_dot_graph())

# print(fixed.compute().as_list())
# print(random.compute().as_list())

# print(fixed.entropy().compute().as_list())
# print(random.entropy().compute().as_list())

# fixed = fixed.expand()
# random = random.expand()

exit(0)
print(fixed.compute().as_list())
fixed = fixed.map(lambda v: np.array(v) * 2)
print(fixed.compute().as_list())
exit(0)

start = time.time()
# print(vg.mutual_information(fixed, random).compute().as_list())
# print(vg.mutual_information(fixed, random).compute().as_list())
result = vg.Array._from_py(vg.mutual_information(fixed, random).compute())
print(result)
print(np.array(result))
print(vg.Array(np.array(result)).as_list())
print(f"Computation time: {time.time() - start:02f}s")
