Mitsubachi is a program to check for bit-rot or unsynchronized files in same directory structures in different places.

It uses a database to keep track of files and its signatures to detect differences in files.

Database schema

table - a table is created for each "root" of directory structure
record - each record in the table is a file. directories are not included as records. symlinks are ... ignored? included?

* path (primary key) - the relative path from the "root"
* basename - base file name of the file; see GNU `basename` command
* dirname - base directory name of the file; see GNU `dirname` command
* signature - a hash from the contents of the file
* timestamp - modification timestamp of file.
* updated - time this record was updated


MIT license, as all libraries used are MIT.