// vogls: backend=bytecode
// vogls: verify-stdout
module t;
  integer seed, r; reg [15:0] lo, hi; reg [99:0] wlo, whi; reg [7:0] m;
  initial begin
    seed = 5; lo = 0; hi = 100; wlo = 100'h1_0000_0000_0000_0003; whi = 100'h1_0000_0000_0000_0050; m = 8'd3;
    r = $dist_uniform(seed, lo, hi); $display("%0d %0d", r, seed);
    r = $dist_normal(seed, lo, hi); $display("%0d %0d", r, seed);
    r = $dist_uniform(seed, wlo, whi); $display("%0d %0d", r, seed);
    r = $dist_poisson(seed, m); $display("%0d %0d", r, seed);
    r = $dist_exponential(seed, m); $display("%0d %0d", r, seed);
  end
endmodule
