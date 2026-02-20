// vogls: skip=two-value-logic
// vogls: verify-stdout
module gate( input i, output [3:0] o );
    assign o = { 4{i} };

    specify
        (i *> o)   = 1, 2, 3;
    endspecify
endmodule

module tb();
    reg i;
    wire [3:0] o;

    gate g(i, o);

    always @(o) #0 $display("[T=%0d] o = %0x",  $time(), o);

    initial begin
        #20 i = 1'b0;
        #20 i = 1'b1;
        #20 i = 1'bx;
        #20 i = 1'bz;
        #20 i = 1'b0;
    end
endmodule
