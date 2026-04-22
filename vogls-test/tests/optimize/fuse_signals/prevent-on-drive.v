// vogls: verify-ir
module m(x);
    inout x;
    initial begin
        #0 x = 1'b1;
    end
endmodule

module tb();
    reg a;
    m _m(a);
endmodule
