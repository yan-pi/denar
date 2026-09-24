; Dead man's switch:
; Before the deadline, only owner can spend.
; After the deadline, heir can spend.
(if (< (tx 1) deadline)
    (bip340_verify owner_pk (bip342_txmsg) owner_sig)
    (bip340_verify heir_pk (bip342_txmsg) heir_sig))
