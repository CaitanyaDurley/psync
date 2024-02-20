[![License: GPLv3](https://img.shields.io/badge/License-GPL%20v3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

This is a fork of  _hweidner_'s psync project. My changes are as follows:
1. Individual files are assigned to worker threads rather than whole directories - this has two benefits:
	* Prevents one thread being assigned a directory full of large files, while the others idle
	* Performs better with shallow directory trees
1. Traversal of the entire source directory tree is assigned to a dedicated thread (once traversal is finished the thread is repurposed) rather than each worker traversing its subdirectory - this ensures:
	* Worker threads are consistently maximising throughput to OSTs
	* MDTs (of which there are likely not many) do not have to deal with contention
1. The default number of threads is 1 (rather than 16) - for sanity when testing on non-distributed filesystems
1. Use Rust instead of Go - because I wanted to

Parallel Sync - parallel recursive copying of directories
=========================================================

psync is a tool which copies a directory recursively to another directory.
Unlike "cp -r", which walks through the files and subdirectories in sequential
order, psync copies several files concurrently by using threads.

A recursive copy of a directory can be a throughput bound or latency bound
operation, depending on the size of files and characteristics of the source
and/or destination file system. When copying between standard file systems on
two local hard disks, the operation is typically throughput bound, and copying
in parallel has no performance advantage over copying sequentially. In this
case, you have a bunch of options, including "cp -r" or "rsync".

However, when copying from or to network file systems (NFS, CIFS), WAN storage
(WebDAV, external cloud services), distributed file systems (GlusterFS, CephFS)
or file systems that live on a DRBD device, the latency for each file access is
often limiting performance factor. With sequential copies, the latencies sum up
and can consume lots of time, although the bandwidth is not saturated. In this
case, it can make up a significant performance boost if the files are copied in
parallel.

In the case of distributed filesystems, multiple copying threads also increases
the likelihood of keeping multiple Object Storage Targets busy. Of course, this
will only come into play if your network has sufficient bandwidth.


Installation
------------

No pre-built executables are provided. You must build from source.
The master branch is where latest changes may be found. Clone a release branch (or download the source code from a tagged release) for production-ready code.
To compile, run:
```
cargo build --release
```
you will find the executable at _target/release/psync_.
Note an internet connection is required to download dependencies. If you need to build on an offline system, download the vendor.zip file from a tagged release and run the same command as before (dependencies are pre-downloaded into the _vendor_ directory).

Usage
-----

psync is invoked as follows:

	psync [-verbose|-quiet] [-threads <num>] [-owner] [-times] [-create] source destination

	-verbose        - verbose mode, prints the current workload to STDOUT
	-quiet          - quiet mode, suppress warnings
	-threads <num>  - number of concurrent threads, 1 <= <num> <= 1024, default 16
	-owner          - preserve ownership (user / group)
	-times          - preserve timestamps (atime / mtime)
	-create         - create destination directory, if needed (with standard permissions)
	source          - source directory
	destination     - destination directory

The behaviour of source & destination are as with `cp -R`. Namely:
1. The source directory must exist and not be a path ending in _._ or _.._
1. If the destination directory does not exist then:
	* The destination's parent directory must exist
	* The destination directory will be created
	* The contents of _source_ will be copied into _destination_
1. If the destination directory does exist then:
	* The source directory itself will be copied into _destination_
	* That is, a directory of the same basename as _source_ will be created within _destination_

Where psync should not be used
------------------------------

Parallel copying is typically not so useful when copying between local or
very fast hard disks. psync can be used there, and with a moderate concurrency
level like 2..5, it can be slightly faster than a sequential copy.

Parallel copying should never be used when duplicating directories on the same
physical hard disk. Even sequential copies suffer from the frequent hard disk head
movements which are needed to read and write concurrently on/to the same disk.
Parallel copying even increases the amount of head movements.

Never use psync when writing to a FAT/VFAT/exFAT file system! Those file systems
are best written sequentially. Parallel write access will be slower, and leads
to inefficient data structures and fragmentation. Reading from those file systems
with psync should be efficient.

Performance values
------------------

Here are some performance values comparing psync to cp and rsync when copying
a large directory structure with many small files from a local file system to
an NFS share.

The NFS server has an AMD E-350 CPU, 8 GB of RAM, a 2TB hard drive (WD Green
series) running Debian GNU/Linux 10 (Linux kernel 4.19). The NFS export is
a logical volume on the HDD with ext4 file system. The NFS export options are:
rw,no_root_squash,async,no_subtree_check.

The client is a workstation with AMD Ryzen 7 1700 CPU, 64 GB of RAM, running
Ubuntu 18.04 LTS with HWE stack (Linux kernel 5.3). The data to copy is located
on a 1TB SATA SSD with XFS, and buffered in memory. The NFS mount options are:
fstype=nfs,vers=3,soft,intr,async.

The hosts are connected over ethernet with 1 Gbit/s, ping latency is 160µs.

The data is an extracted linux kernel source code 4.15.2 tarball, containing
62273 files and 32 symbolic links in 4377 directories, summing up to 892 MB
(as seen by "du -s"). It is copied from the workstation to the server over NFS.

The options for the three commands are selected comparably. They copy the files
and links recursively and preserve permissions, but no ownership or time stamps.

	Command                Estimated time  Throughput
	=================================================
    cp -r SRC DEST              1m50,288s   8,09 MB/s
    rsync -rl SRC/ DEST/        3m05,479s   4,81 MB/s
    psync SRC DEST              0m23,398s  38,12 MB/s

Limits and TODOs
----------------

psync currently can only handle directories, regular files, and symbolic links.
Other filesystem entries like devices, sockets or named pipes are silently ignored.
A warning is printed when trying to copy such special files.

psync preserves the Unix permissions (rwx) of the copied files and directories,
but has currently no support for preserving other permission bits (suid, sticky).

When using the according options, psync tries to preserve the ownership
(user/group) and/or the access and modification time stamps. Preserve ownership
does only work when psync is running under the root user account. Preserving the
time stamps does only work for regular files and directories, not for symbolic
links.

psync does currently implement a simple recursive copy, like "cp -r", and not
a versatile sync algorithm like rsync. There is no check wether a file already
exists in the destination, nor its content and timestamps. Existing files on the
destination side are not deleted when they don't exist on the source side.

psync is being developed under Linux (Debian, Ubuntu, CentOS). It should work on
other distributions, but this has not been tested. It does currently not compile
for Windows, Darwin (MacOS), NetBSD and FreeBSD (but this should easily be
fixed).

Contributing
------------

Please see [CONTRIBUTING.md](CONTRIBUTING.md) on how to contribute to the
development of psync.

License
-------

psync is released under the terms of the GNU General Public License, version 3.
