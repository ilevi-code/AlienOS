#!/bin/bash
DEV=$1
FILE=$2
MAJOR=$3

DIR=$(dirname $FILE)
TEMP_FILE=$(mktemp)
e2cp $TEMP_FILE $DEV:$FILE
rm $TEMP_FILE
INODE=$(e2ls $DEV:$FILE -i | tr -s ' ' | cut -f2 -d' ')
echo $INODE

FIRST_INODE_OFFSET=$((5*1024))
INODE_SIZE=128
# Using block[0] as major
DEV_MAJOR_OFFSET=40
FILE_MODE_OFFSET=0

# convert to little-endian
FORMATTED_MAJOR=$(printf "%08x" $MAJOR | sed -E 's/(..)(..)(..)(..)/\\x\4\\x\3\\x\2\\x\1/')
# Patch the the block table
SEEK=$((FIRST_INODE_OFFSET + INODE_SIZE*(INODE-1) + DEV_MAJOR_OFFSET))
printf $FORMATTED_MAJOR | dd of=$DEV bs=1 seek=$SEEK conv=notrunc

# Patch the mode to be char-dev + 0o644
SEEK=$((FIRST_INODE_OFFSET + INODE_SIZE*(INODE-1) + FILE_MODE_OFFSET))
printf '\xa4\x21' | dd of=$DEV bs=1 seek=$SEEK conv=notrunc
