// vogls: verify-stdout
module x();
    integer i;
    initial for (i = 2; i < 5; i = i + 1) $display(i);
endmodule
