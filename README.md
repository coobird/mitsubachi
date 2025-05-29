# About Mitsubachi

Mitsubachi is a simple program to check for bit-rot or unsynchronized files in same directory structures in different places.

This program was created to check the contents of two copies of backup directories residing in different physical disks.
(Copies created by `rsync`.)

It uses a database to keep track of files and its signatures to detect differences in files.
A difference can mean either there's been a change to the file -- whether an intentional update or some error like bit-rot.

**Mitsubachi is not a substitute for a software and/or hardware solution, such as ZFS, that performs integrity checks on data.**
See the **Limitations** section for more details.

**USE AT YOUR OWN RISK.**

# Requirements

* SQLite

# Limitations

A major limitation is that Mitsubachi does not read the underlying storage directly. 
Therefore, any discrepancies between actual storage and what's reported by the operating system (such as cache) could cause erroneous results.
This also extends to memory errors (e.g. bit flips in DRAM) that could lead to erroneous results.

In other words, **you are strongly advised not to use Mitsubachi as the sole tool for data integrity checks.**

# License

Mitsubachi is distributed under the terms of the MIT license.

Refer to the `LICENSE` file for more details.

# Appendix

The name _Mitsubachi_ comes from the Japanese word for _honey bee_.
Since this program "buzzes" around the file system and "collects" hashes, it's behavior is similar to honey bees collecting nectar.