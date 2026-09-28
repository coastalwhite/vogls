module top_module(
    input a,
    input b,
    input sel_b1,
    input sel_b2,
    output wire out_assign,
    output reg out_always   ); 

    assign out_assign = (sel_b1 & sel_b2) ? b : a;
    always @(*) begin
        if (sel_b1 & sel_b2) out_always = b;
        else                 out_always = a;
    end

endmodule

module tb();
    reg a, b, sel_b1, sel_b2;
    wire out_assign, out_always;

    top_module m(a, b, sel_b1, sel_b2, out_assign, out_always);

    integer i;
    initial begin
        #1 a = 0; b = 0; sel_b1 = 0; sel_b2 = 0;
        #1 $vogls_assert_eq(out_assign, 0); $vogls_assert_eq(out_always, 0);

        #1 a = 0; b = 1; sel_b1 = 0; sel_b2 = 0;
        #1 $vogls_assert_eq(out_assign, 0); $vogls_assert_eq(out_always, 0);

        #1 a = 0; b = 1; sel_b1 = 1; sel_b2 = 1;
        #1 $vogls_assert_eq(out_assign, 1); $vogls_assert_eq(out_always, 1);

        #1 a = 1; b = 1; sel_b1 = 1; sel_b2 = 1;
        #1 $vogls_assert_eq(out_assign, 1); $vogls_assert_eq(out_always, 1);

        #1 a = 1; b = 1; sel_b1 = 1; sel_b2 = 0;
        #1 $vogls_assert_eq(out_assign, 1); $vogls_assert_eq(out_always, 1);

        #1 a = 1; b = 0; sel_b1 = 1; sel_b2 = 0;
        #1 $vogls_assert_eq(out_assign, 1); $vogls_assert_eq(out_always, 1);

        for (i = 0; i < 16; i = i + 1) begin
            #1
            a = i[3]; b = i[2]; sel_b1 = i[1]; sel_b2 = i[0];
            #1
            $vogls_assert_eq(out_assign, out_always);
        end
    end
endmodule
