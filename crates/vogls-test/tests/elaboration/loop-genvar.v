module tb();
    genvar i;
    for (i = 0; i < 1; i = i + 1) begin
        localparam j = i;
    end
endmodule
