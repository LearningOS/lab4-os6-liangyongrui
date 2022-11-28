//! File and filesystem-related syscalls

use crate::fs::increase_nlink;
use crate::fs::open_file;
use crate::fs::OpenFlags;
use crate::fs::Stat;
use crate::fs::ROOT_INODE;
use crate::mm::translated_byte_buffer;
use crate::mm::translated_refmut;
use crate::mm::translated_str;
use crate::mm::UserBuffer;
use crate::task::current_task;
use crate::task::current_user_token;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

// YOUR JOB: 扩展 easy-fs 和内核以实现以下三个 syscall
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    let st = translated_refmut(current_user_token(), st);
    let tcb = current_task().unwrap();
    let inner = tcb.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    let Some(ref inode) = inner.fd_table[fd] else { return -1; };
    *st = inode.status();
    0
}

pub fn sys_linkat(old_name: *const u8, new_name: *const u8) -> isize {
    let token = current_user_token();
    let old_name = translated_str(token, old_name);
    let new_name = translated_str(token, new_name);
    if ROOT_INODE.find(&new_name).is_some() || increase_nlink(&old_name, &new_name).is_none() {
        return -1;
    }
    0
}

pub fn sys_unlinkat(name: *const u8) -> isize {
    let token = current_user_token();
    let name = translated_str(token, name);
    let (success, clear_inode) =
        ROOT_INODE.modify_disk_inode(|disk_inode| ROOT_INODE.unlink(disk_inode, &name));
    if success {
        // if let Some(inode) = clear_inode {
        //     inode.clear();
        // }
        return 0;
    }
    -1
}
