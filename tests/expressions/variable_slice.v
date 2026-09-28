module tb;
  reg [255:0] v; reg [127:0] a; integer s;
  initial begin
    v = {128'h1234, 128'hdeadacdb};
    s = 1;
    a = v[128*s +: 128];        // variable-offset part-select, >64 bits
    $vogls_assert_eq(a, 128'h1234);
  end
endmodule
