// vogls: verify-vcd
module tb();
    reg x;

    initial begin
        x = 0;
        #5
        x = 1;
        #3
        x = 0;
        #7
        x = 1;
        #1
        $finish();
    end
endmodule
