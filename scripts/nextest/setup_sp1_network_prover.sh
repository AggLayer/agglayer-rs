#!/bin/sh

# Exit with 1 if NEXTEST_ENV isn't defined.
if [ -z "$NEXTEST_ENV" ]; then
    exit 1
fi

echo "SP1_PRIVATE_KEY=dcf2cbdd171a21c480aa7f53d77f31bb102282b3ff099c78e3118b37348c72f7" >> "$NEXTEST_ENV"
