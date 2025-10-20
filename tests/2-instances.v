module top_level();
    reg i_a1, i_a2;

    a a1(i_a1);
    a a2(i_a2);

    initial begin
        i_a1 <= 0;
        i_a2 <= 0;

        #1
        i_a1 <= 1;
        #1
        i_a2 <= 1;

        #1
        i_a1 <= 0;
        i_a2 <= 0;

        #1
        i_a1 <= 1;
        i_a2 <= 1;
    end
endmodule

module a(
    input i_a
);
    always @ (posedge i_a) $display("Hello!");
endmodule