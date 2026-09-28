// vogls: verify-stdout
`timescale 1ps / 1ps
module x();
    reg A;
    initial begin
        #3
        A = 1;
        #7
        $display("lupdt = %0d", $vogls_lupdt(A));
        A = 0;
        $display("lupdt = %0d", $vogls_lupdt(A));
        #7
        $display("lupdt = %0d", $vogls_lupdt(A));
    end
endmodule
