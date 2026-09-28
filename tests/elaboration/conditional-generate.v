module top();
	wire a;

    generate
        if (1'b0) begin: slow
            wire x = 1'b0;
        end
        else if (1'b1) begin: fast
            wire x = 1'b1;
        end
    endgenerate

    generate
        if (1'b0) begin: slow2
            assign a = slow.x;
        end
        else if (1'b1) begin: fast2
            assign a = fast.x;
        end
    endgenerate

    initial begin
        #0
        $vogls_assert_eq(a, 1'b1);
    end
endmodule
