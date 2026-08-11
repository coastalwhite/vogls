// vogls: verify-stdout
`define MIN(a, b) ((a < b) ? (a) : (b))
`define ID(a) a
`define FIVETIMES 5*

module x();
initial begin
    $display("MIN      %0d", `MIN(5, 10));
    $display("MIN      %0d", `MIN(7, 3));
    $display("MIN      %0d", `MIN({7, 6}, 3));
    $display("ID       %0d", `ID(7));
    `ID()
    $display("MIN(MIN) %0d", `MIN(`MIN(8, 1), 3));
    $display("F        %0d", `FIVETIMES(1));
end
endmodule
