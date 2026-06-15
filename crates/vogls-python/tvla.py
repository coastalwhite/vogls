import vogls as vg
from random import randint

CYCLE = 2
N_CYCLES = 8

NUM_RUNS = 100

def run(*, random: bool) -> vg.vgr.PyLazyRunVector:
    design = vg.LazyDesign("xor.v")
    r = design.run()

    if random:
        r = r.set_signal("a", vg.LazyArray.random_bits(NUM_RUNS, 32, seed=randint(0, 100)))
        r = r.set_signal("b", vg.LazyArray.random_bits(NUM_RUNS, 32, seed=randint(0, 100)))

    hd = r.trace_start().run_for(CYCLE * N_CYCLES).hamming_distance()

    dist = hd.get("hamming_distance.dist").extract_run_vector()
    time = hd.get("hamming_distance.time").extract_run_vector()

    return dist.window_sum(by=time, width=CYCLE, start=0, end=CYCLE * N_CYCLES)

def masked_run(*, random: bool) -> vg.vgr.PyLazyRunVector:
    design = vg.LazyDesign("masked_xor.v")
    r = design.run()

    r = r.set_signal("a_1", vg.LazyArray.random_bits(NUM_RUNS, 32, seed=randint(0, 100)))
    r = r.set_signal("b_1", vg.LazyArray.random_bits(NUM_RUNS, 32, seed=randint(0, 100)))

    if random:
        r = r.set_signal("a_0", vg.LazyArray.random_bits(NUM_RUNS, 32, seed=randint(0, 100)))

    hd = r.trace_start().run_for(CYCLE * N_CYCLES).hamming_distance()

    dist = hd.get("hamming_distance.dist").extract_run_vector()
    time = hd.get("hamming_distance.time").extract_run_vector()

    return dist.window_sum(by=time, width=CYCLE, start=0, end=CYCLE * N_CYCLES)


fixed = masked_run(random=False)
random = masked_run(random=True)

print(fixed.compute().as_list())
print(random.compute().as_list())

print(vg.t_test(fixed, random).compute().as_list())
