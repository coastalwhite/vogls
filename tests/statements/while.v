module x();
    integer i;
    initial begin
        i = 0;
        while(i < 5) begin
            $display("Hello!");
            i = i + 1;
        end
    end
endmodule
