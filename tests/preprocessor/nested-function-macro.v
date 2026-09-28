// vogls: fail=lex
// This is not yet implemented. This test makes sure, it gives a proper error
// instead of giving garbage.
`define ADD(a, b) ((a) + (b))
`define SUM(x, y) `ADD(x, y)

module x();
initial begin
    $display("SUM      %0d", `SUM(5, 7));
end
endmodule
