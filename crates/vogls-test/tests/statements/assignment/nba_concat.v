// vogls: verify-stdout
module x();
    reg [31:0] a = 0;

    initial begin
        #0
        a <= 4;
        #1
        a <= 0;
        #1
        a <= 4;
    end

    always @* begin
        $display("a = %h", {a[31:2], 2'b00});
    end
endmodule
