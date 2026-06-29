import vogls as vg
import os
import matplotlib.pyplot as plt

DUT = "../vogls-test/tests/integration/aoki-aes/lut.v"

CYCLE = 10
NUM_RUNS = 1000
defines = []
# defines += ['AES_IMPL_COMP']

KEY = vg.LazyValue.random_bits(128, seed=42).compute()


def run(*, random: bool) -> vg.LazyRunVector:
    design = vg.LazyDesign(
        DUT,
        top_level_module="tb",
        defines=defines,
    )
    r = design.run()

    # Set the key to a new random value each run or constant.
    if random:
        r = r.set_signal("Key", vg.LazyArray.random_bits(NUM_RUNS, 128, seed=510))
    else:
        r = r.set_signal("Key", KEY.repeat(NUM_RUNS).lazy())

    r = r.trace_start()           # Start tracing all the wires and registers
    r = r.run_for(22 * CYCLE)     # Run for 22 cycles
    r = r.hamming_distance("hd")  # Get the Hamming distance from the trace
    r = r.finish()                # Finish the running

    # Extract and collapse the Hamming distance.
    hd = r.get("hd", vg.LazyPlan)
    d = hd.get("dist", vg.LazyRunVector)
    t = hd.get("time", vg.LazyRunVector)
    return d.window_sum(by=t, width=CYCLE, start=6 * CYCLE, end=22 * CYCLE)


fixed = run(random=False)
random = run(random=True)

tvla = vg.welch_t_test(fixed, random)

# Plan is finished -> Start computing the result
result = tvla.compute()
print(result.as_list())


# Plot the result
plt.plot(range(6, 22), result)
plt.title("TVLA per Cycle")
plt.xlabel("Cycle")
plt.ylabel("T-Test score")
plt.axhline(y=4.5, color="r", linestyle="-")
plt.axhline(y=-4.5, color="r", linestyle="-")
plt.tight_layout()
plt.savefig("out.svg")
os.system("inkview out.svg")


with open("plan.dot", "w") as f:
    f.write(tvla.to_dot_graph())
os.system("dot -o plan.svg -Tsvg plan.dot")
os.system("inkview plan.svg")
