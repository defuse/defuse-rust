#!/bin/bash
# makepaste.sh - encrypt text with gpg and host it on defuse.ca's pastebin.
#
# This script reads text from standard input, encrypts it with a random password
# using gpg, uploads it to defuse.ca's pastebin, then prints the command to
# download and decrypt it.
#
# https://defuse.ca/pastebin.htm

PASSWORD=$(gpg --gen-random 2 16 | base64)
URL=$(                                                                  \
        gpg --batch --passphrase-fd 3 -c -a 3<<<"$PASSWORD" |          \
        curl -s -d "jscrypt=no" -d "lifetime=864000"                    \
        -d "shorturl=yes" --data-urlencode "paste@-"                    \
        https://defuse.ca/bin/add.php -D - |                            \
        grep -i location | cut -d " " -f 2 | tr -d '\r\n'              \
)
echo "P='$PASSWORD'; gpg -d -q --batch --passphrase-fd 0 <(wget https://defuse.ca$URL?raw=true -q -O -) < <(printf '%s\n' \"\$P\")"

