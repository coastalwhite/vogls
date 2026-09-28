module x();
    wire [31:0] regs [0:32];
    initial begin
        regs[1] = 32'h3FC;
        $vogls_assert_eq(regs[5'h01], 32'h3FC);
    end
endmodule
