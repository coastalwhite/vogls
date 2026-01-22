// vogls: verify-stdout
module x();
    localparam D = 1;
    case (1'b0)
        1'b1: begin
            initial $display("Should not be printed");
            initial $vogls_assert_eq(1'b0, 1'b1);
        end
        1'b0: begin
            initial $display("Should be printed");
            initial $vogls_assert_eq(1'b1, 1'b1);
        end
    endcase

    case (D)
        1'b1: begin
            initial $display("Should be printed");
            initial $vogls_assert_eq(1'b1, 1'b1);
        end
        1'b0: begin
            initial $display("Should not be printed");
            initial $vogls_assert_eq(1'b0, 1'b1);
        end
        default: begin
            initial $display("%d", D);
        end
    endcase
endmodule
