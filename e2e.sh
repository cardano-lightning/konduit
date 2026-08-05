# admin tx deploy
# consumer tx --open "deadbeef,$(adaptor show constants),20"
# consumer tx --open "fade,$(adaptor show constants),50"
## The below could require an extra delay
# consumer tx --add "deadbeef,200"

export SECRET="0000000000000000000000000000000000000000000000000000000000000000"
# adaptor tx --receipt "$(consumer show keytag deadbeef);$(consumer make squash --tag deadbeef --amount 4000000 --index 3);$(consumer make locked --tag deadbeef --index 4 --amount 1000000 --duration 8h --secret $SECRET),$SECRET"

export KEYTAG_DEADBEEF=$(consumer show keytag deadbeef)
export SQUASH_DEADBEEF=$(consumer make squash --tag deadbeef --amount 8000000 --index 5)
export LOCKED_DEADBEEF=$(consumer make locked --tag deadbeef --index 6 --amount 2000000 --duration 8h --secret $SECRET)
export KEYTAG_FADE=$(consumer show keytag fade)
export SQUASH_FADE=$(consumer make squash --tag fade --amount 5000000 --index 5)
export LOCKED_FADE=$(consumer make locked --tag fade --index 6 --amount 1000000 --duration 8h --secret $SECRET)

# adaptor tx --receipt "$KEYTAG_DEADBEEF;$SQUASH_DEADBEEF;$LOCKED_DEADBEEF,$SECRET" --receipt "$KEYTAG_FADE;$SQUASH_FADE;$LOCKED_FADE,$SECRET"
# consumer tx --close fade"
# consumer tx --open "babe,$(adaptor show constants),80"

export KEYTAG_BABE=$(consumer show keytag babe)
# export SQUASH_BABE=$(consumer make squash --tag babe --amount 8000000 --index 5)
# # Let's drain deadbeef
# export SQUASH_DEADBEEF2=$(consumer make squash --tag deadbeef --amount 210000000 --index 7)
# adaptor tx --receipt "$KEYTAG_BABE;$SQUASH_BABE" --receipt "$KEYTAG_DEADBEEF;$SQUASH_DEADBEEF2"


export SQUASH_BABE=$(consumer make squash --tag babe --amount 10000000 --index 7)
# adaptor tx --receipt "$KEYTAG_BABE;$SQUASH_BABE"

consumer tx --add "babe,150"
