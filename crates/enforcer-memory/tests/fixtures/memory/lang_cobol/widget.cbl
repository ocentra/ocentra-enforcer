       IDENTIFICATION DIVISION.
       PROGRAM-ID. WIDGET.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       COPY COMMONDEF.
       01 X PIC 9(4).
       PROCEDURE DIVISION.
       MAIN-PARA.
           PERFORM SUB-PARA.
           CALL 'HELPER' USING X.
           IF X > 0
               DISPLAY "POSITIVE"
           END-IF.
           EVALUATE X
               WHEN 1
                   DISPLAY "ONE"
               WHEN OTHER
                   DISPLAY "OTHER"
           END-EVALUATE.
           STOP RUN.
       SUB-PARA.
           DISPLAY "IN SUB".
