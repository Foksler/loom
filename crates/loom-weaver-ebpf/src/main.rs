// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

// SECURITY: All event structs MUST be fully zero-initialized before
// populating fields to prevent leaking uninitialized kernel memory.

#![no_std]
#![no_main]

use aya_ebpf::{
	macros::{map, tracepoint},
	maps::RingBuf,
	programs::TracePointContext,
};
use aya_log_ebpf::info;
use loom_weaver_ebpf::{create_event_header, get_pid_tgid, read_str_from_user, should_capture_event};
use loom_weaver_ebpf_common::{
	ConnectEvent, EscapeType, EventType, FileOpenEvent, MemoryExecEvent, PrivilegeChangeEvent,
	ProcessExecEvent, ProcessExitEvent, ProcessForkEvent, SandboxEscapeEvent, MAX_COMM_LEN,
	MAX_PATH_LEN,
};

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

#[tracepoint]
pub fn sys_enter_execve(ctx: TracePointContext) -> u32 {
	match try_sys_enter_execve(ctx) {
		Ok(()) => 0,
		Err(_) => 1,
	}
}

fn try_sys_enter_execve(ctx: TracePointContext) -> Result<(), i64> {
	if !should_capture_event() {
		return Ok(());
	}
	let header = create_event_header(EventType::ProcessExec);

	let filename_ptr: *const u8 = unsafe { ctx.read_at(16)? };

	let mut event = ProcessExecEvent {
		header,
		filename: [0u8; MAX_PATH_LEN],
		filename_len: 0,
		ret: 0,
	};

	unsafe {
		if let Ok(len) = read_str_from_user(&ctx, filename_ptr, &mut event.filename) {
			event.filename_len = len as u32;
		}
	}

	if let Some(mut buf) = EVENTS.reserve::<ProcessExecEvent>(0) {
		unsafe {
			buf.as_mut_ptr().write(event);
		}
		buf.submit(0);
	}

	info!(&ctx, "execve: pid={}", header.pid);
	Ok(())
}

#[tracepoint]
pub fn sys_exit_execve(ctx: TracePointContext) -> u32 {
	match try_sys_exit_execve(ctx) {
		Ok(()) => 0,
		Err(_) => 1,
	}
}

fn try_sys_exit_execve(ctx: TracePointContext) -> Result<(), i64> {
	if !should_capture_event() {
		return Ok(());
	}
	let ret: i64 = unsafe { ctx.read_at(16)? };
	let (pid, _tgid) = get_pid_tgid();

	info!(&ctx, "execve exit: pid={} ret={}", pid, ret);
	Ok(())
}

#[tracepoint]
pub fn sys_enter_openat(ctx: TracePointContext) -> u32 {
	match try_sys_enter_openat(ctx) {
		Ok(()) => 0,
		Err(_) => 1,
	}
}

fn try_sys_enter_openat(ctx: TracePointContext) -> Result<(), i64> {
	if !should_capture_event() {
		return Ok(());
	}
	let header = create_event_header(EventType::FileOpen);

	let dirfd: i32 = unsafe { ctx.read_at(16)? };
	let filename_ptr: *const u8 = unsafe { ctx.read_at(24)? };
	let flags: i32 = unsafe { ctx.read_at(32)? };

	let mut event = FileOpenEvent {
		header,
		dirfd,
		flags,
		filename: [0u8; MAX_PATH_LEN],
		filename_len: 0,
	};

	unsafe {
		if let Ok(len) = read_str_from_user(&ctx, filename_ptr, &mut event.filename) {
			event.filename_len = len as u32;
		}
	}

	if let Some(mut buf) = EVENTS.reserve::<FileOpenEvent>(0) {
		unsafe {
			buf.as_mut_ptr().write(event);
		}
		buf.submit(0);
	}

	info!(&ctx, "openat: pid={} dirfd={} flags={}", header.pid, dirfd, flags);
	Ok(())
}

#[tracepoint]
pub fn sys_enter_connect(ctx: TracePointContext) -> u32 {
	match try_sys_enter_connect(ctx) {
		Ok(()) => 0,
		Err(_) => 1,
	}
}

fn try_sys_enter_connect(ctx: TracePointContext) -> Result<(), i64> {
	if !should_capture_event() {
		return Ok(());
	}
	let header = create_event_header(EventType::NetworkConnect);

	let sockfd: i32 = unsafe { ctx.read_at(16)? };
	let addr_ptr: *const u8 = unsafe { ctx.read_at(24)? };
	let addrlen: u32 = unsafe { ctx.read_at(32)? };

	let mut event = ConnectEvent { header, sockfd, addrlen, addr: [0u8; 128], family: 0, port: 0 };

	let read_len = if addrlen as usize > 128 { 128 } else { addrlen as usize };

	unsafe {
		if aya_ebpf::helpers::bpf_probe_read_user(
			event.addr.as_mut_ptr() as *mut _,
			read_len as u32,
			addr_ptr as *const _,
		)
		.is_ok()
		{
			if read_len >= 2 {
				event.family = u16::from_ne_bytes([event.addr[0], event.addr[1]]);
			}
			if read_len >= 4 && (event.family == 2 || event.family == 10) {
				event.port = u16::from_be_bytes([event.addr[2], event.addr[3]]);
			}
		}
	}

	if let Some(mut buf) = EVENTS.reserve::<ConnectEvent>(0) {
		unsafe {
			buf.as_mut_ptr().write(event);
		}
		buf.submit(0);
	}

	info!(&ctx, "connect: pid={} sockfd={} family={}", header.pid, sockfd, event.family);
	Ok(())
}

// =============================================================================
// Process events
// =============================================================================

#[tracepoint]
pub fn sys_enter_clone(ctx: TracePointContext) -> u32 {
	match try_sys_enter_clone(ctx) {
		Ok(()) => 0,
		Err(_) => 1,
	}
}

fn try_sys_enter_clone(ctx: TracePointContext) -> Result<(), i64> {
	if !should_capture_event() {
		return Ok(());
	}
	let header = create_event_header(EventType::ProcessFork);

	let clone_flags: u64 = unsafe { ctx.read_at(16)? };

	let event = ProcessForkEvent {
		header,
		parent_pid: header.pid,
		child_pid: 0,
		clone_flags,
	};

	if let Some(mut buf) = EVENTS.reserve::<ProcessForkEvent>(0) {
		unsafe {
			buf.as_mut_ptr().write(event);
		}
		buf.submit(0);
	}

	info!(&ctx, "clone: pid={} flags={}", header.pid, clone_flags);
	Ok(())
}

#[tracepoint]
pub fn sys_exit_exit_group(ctx: TracePointContext) -> u32 {
	match try_sys_exit_exit_group(ctx) {
		Ok(()) => 0,
		Err(_) => 1,
	}
}

fn try_sys_exit_exit_group(ctx: TracePointContext) -> Result<(), i64> {
	if !should_capture_event() {
		return Ok(());
	}
	let header = create_event_header(EventType::ProcessExit);

	let exit_code: i32 = unsafe { ctx.read_at(16)? };

	let event = ProcessExitEvent {
		header,
		exit_code,
		signal: 0,
		comm: [0u8; MAX_COMM_LEN],
	};

	if let Some(mut buf) = EVENTS.reserve::<ProcessExitEvent>(0) {
		unsafe {
			buf.as_mut_ptr().write(event);
		}
		buf.submit(0);
	}

	info!(&ctx, "exit_group: pid={} code={}", header.pid, exit_code);
	Ok(())
}

// =============================================================================
// Privilege change events
// =============================================================================

const PRIV_CHANGE_SETUID: u32 = 1;
const PRIV_CHANGE_SETGID: u32 = 2;
const PRIV_CHANGE_PTRACE: u32 = 3;

#[tracepoint]
pub fn sys_enter_setuid(ctx: TracePointContext) -> u32 {
	match try_sys_enter_setuid(ctx) {
		Ok(()) => 0,
		Err(_) => 1,
	}
}

fn try_sys_enter_setuid(ctx: TracePointContext) -> Result<(), i64> {
	if !should_capture_event() {
		return Ok(());
	}
	let header = create_event_header(EventType::PrivilegeChange);

	let new_uid: u32 = unsafe { ctx.read_at(16)? };

	let event = PrivilegeChangeEvent {
		header,
		old_uid: header.uid,
		new_uid,
		old_gid: header.gid,
		new_gid: header.gid,
		old_euid: header.uid,
		new_euid: new_uid,
		capability: 0,
		change_type: PRIV_CHANGE_SETUID,
	};

	if let Some(mut buf) = EVENTS.reserve::<PrivilegeChangeEvent>(0) {
		unsafe {
			buf.as_mut_ptr().write(event);
		}
		buf.submit(0);
	}

	info!(&ctx, "setuid: pid={} new_uid={}", header.pid, new_uid);
	Ok(())
}

#[tracepoint]
pub fn sys_enter_setgid(ctx: TracePointContext) -> u32 {
	match try_sys_enter_setgid(ctx) {
		Ok(()) => 0,
		Err(_) => 1,
	}
}

fn try_sys_enter_setgid(ctx: TracePointContext) -> Result<(), i64> {
	if !should_capture_event() {
		return Ok(());
	}
	let header = create_event_header(EventType::PrivilegeChange);

	let new_gid: u32 = unsafe { ctx.read_at(16)? };

	let event = PrivilegeChangeEvent {
		header,
		old_uid: header.uid,
		new_uid: header.uid,
		old_gid: header.gid,
		new_gid,
		old_euid: header.uid,
		new_euid: header.uid,
		capability: 0,
		change_type: PRIV_CHANGE_SETGID,
	};

	if let Some(mut buf) = EVENTS.reserve::<PrivilegeChangeEvent>(0) {
		unsafe {
			buf.as_mut_ptr().write(event);
		}
		buf.submit(0);
	}

	info!(&ctx, "setgid: pid={} new_gid={}", header.pid, new_gid);
	Ok(())
}

#[tracepoint]
pub fn sys_enter_ptrace(ctx: TracePointContext) -> u32 {
	match try_sys_enter_ptrace(ctx) {
		Ok(()) => 0,
		Err(_) => 1,
	}
}

fn try_sys_enter_ptrace(ctx: TracePointContext) -> Result<(), i64> {
	if !should_capture_event() {
		return Ok(());
	}
	let header = create_event_header(EventType::PrivilegeChange);

	let request: i64 = unsafe { ctx.read_at(16)? };
	let target_pid: i64 = unsafe { ctx.read_at(24)? };

	let event = PrivilegeChangeEvent {
		header,
		old_uid: header.uid,
		new_uid: header.uid,
		old_gid: header.gid,
		new_gid: header.gid,
		old_euid: header.uid,
		new_euid: header.uid,
		capability: target_pid as u32,
		change_type: PRIV_CHANGE_PTRACE,
	};

	if let Some(mut buf) = EVENTS.reserve::<PrivilegeChangeEvent>(0) {
		unsafe {
			buf.as_mut_ptr().write(event);
		}
		buf.submit(0);
	}

	info!(&ctx, "ptrace: pid={} request={} target={}", header.pid, request, target_pid);
	Ok(())
}

// =============================================================================
// Memory events (filtered for PROT_EXEC)
// =============================================================================

const PROT_EXEC: u32 = 0x4;

#[tracepoint]
pub fn sys_enter_mmap(ctx: TracePointContext) -> u32 {
	match try_sys_enter_mmap(ctx) {
		Ok(()) => 0,
		Err(_) => 1,
	}
}

fn try_sys_enter_mmap(ctx: TracePointContext) -> Result<(), i64> {
	if !should_capture_event() {
		return Ok(());
	}
	let addr: u64 = unsafe { ctx.read_at(16)? };
	let len: u64 = unsafe { ctx.read_at(24)? };
	let prot: u32 = unsafe { ctx.read_at(32)? };
	let flags: u32 = unsafe { ctx.read_at(40)? };
	let fd: i32 = unsafe { ctx.read_at(48)? };

	if prot & PROT_EXEC == 0 {
		return Ok(());
	}

	let header = create_event_header(EventType::MemoryExec);

	let event = MemoryExecEvent {
		header,
		addr,
		len,
		prot,
		flags,
		fd,
		path: [0u8; MAX_PATH_LEN],
		path_len: 0,
	};

	if let Some(mut buf) = EVENTS.reserve::<MemoryExecEvent>(0) {
		unsafe {
			buf.as_mut_ptr().write(event);
		}
		buf.submit(0);
	}

	info!(&ctx, "mmap: pid={} addr={} len={} prot={}", header.pid, addr, len, prot);
	Ok(())
}

#[tracepoint]
pub fn sys_enter_mprotect(ctx: TracePointContext) -> u32 {
	match try_sys_enter_mprotect(ctx) {
		Ok(()) => 0,
		Err(_) => 1,
	}
}

fn try_sys_enter_mprotect(ctx: TracePointContext) -> Result<(), i64> {
	if !should_capture_event() {
		return Ok(());
	}
	let addr: u64 = unsafe { ctx.read_at(16)? };
	let len: u64 = unsafe { ctx.read_at(24)? };
	let prot: u32 = unsafe { ctx.read_at(32)? };

	if prot & PROT_EXEC == 0 {
		return Ok(());
	}

	let header = create_event_header(EventType::MemoryExec);

	let event = MemoryExecEvent {
		header,
		addr,
		len,
		prot,
		flags: 0,
		fd: -1,
		path: [0u8; MAX_PATH_LEN],
		path_len: 0,
	};

	if let Some(mut buf) = EVENTS.reserve::<MemoryExecEvent>(0) {
		unsafe {
			buf.as_mut_ptr().write(event);
		}
		buf.submit(0);
	}

	info!(&ctx, "mprotect: pid={} addr={} len={} prot={}", header.pid, addr, len, prot);
	Ok(())
}

// =============================================================================
// Sandbox escape detection (critical events)
// =============================================================================

const SYSCALL_UNSHARE: u32 = 272;
const SYSCALL_SETNS: u32 = 308;
const SYSCALL_MOUNT: u32 = 165;

#[tracepoint]
pub fn sys_enter_unshare(ctx: TracePointContext) -> u32 {
	match try_sys_enter_unshare(ctx) {
		Ok(()) => 0,
		Err(_) => 1,
	}
}

fn try_sys_enter_unshare(ctx: TracePointContext) -> Result<(), i64> {
	if !should_capture_event() {
		return Ok(());
	}
	let header = create_event_header(EventType::SandboxEscape);

	let flags: u64 = unsafe { ctx.read_at(16)? };

	let event = SandboxEscapeEvent {
		header,
		escape_type: EscapeType::Namespace as u32,
		syscall_nr: SYSCALL_UNSHARE,
		arg0: flags,
		arg1: 0,
		arg2: 0,
		context: [0u8; MAX_PATH_LEN],
		context_len: 0,
	};

	if let Some(mut buf) = EVENTS.reserve::<SandboxEscapeEvent>(0) {
		unsafe {
			buf.as_mut_ptr().write(event);
		}
		buf.submit(0);
	}

	info!(&ctx, "unshare: pid={} flags={}", header.pid, flags);
	Ok(())
}

#[tracepoint]
pub fn sys_enter_setns(ctx: TracePointContext) -> u32 {
	match try_sys_enter_setns(ctx) {
		Ok(()) => 0,
		Err(_) => 1,
	}
}

fn try_sys_enter_setns(ctx: TracePointContext) -> Result<(), i64> {
	if !should_capture_event() {
		return Ok(());
	}
	let header = create_event_header(EventType::SandboxEscape);

	let fd: i32 = unsafe { ctx.read_at(16)? };
	let nstype: i32 = unsafe { ctx.read_at(24)? };

	let event = SandboxEscapeEvent {
		header,
		escape_type: EscapeType::Namespace as u32,
		syscall_nr: SYSCALL_SETNS,
		arg0: fd as u64,
		arg1: nstype as u64,
		arg2: 0,
		context: [0u8; MAX_PATH_LEN],
		context_len: 0,
	};

	if let Some(mut buf) = EVENTS.reserve::<SandboxEscapeEvent>(0) {
		unsafe {
			buf.as_mut_ptr().write(event);
		}
		buf.submit(0);
	}

	info!(&ctx, "setns: pid={} fd={} nstype={}", header.pid, fd, nstype);
	Ok(())
}

#[tracepoint]
pub fn sys_enter_mount(ctx: TracePointContext) -> u32 {
	match try_sys_enter_mount(ctx) {
		Ok(()) => 0,
		Err(_) => 1,
	}
}

fn try_sys_enter_mount(ctx: TracePointContext) -> Result<(), i64> {
	if !should_capture_event() {
		return Ok(());
	}
	let header = create_event_header(EventType::SandboxEscape);

	let source_ptr: *const u8 = unsafe { ctx.read_at(16)? };
	let target_ptr: *const u8 = unsafe { ctx.read_at(24)? };
	let fstype_ptr: *const u8 = unsafe { ctx.read_at(32)? };

	let mut event = SandboxEscapeEvent {
		header,
		escape_type: EscapeType::Mount as u32,
		syscall_nr: SYSCALL_MOUNT,
		arg0: source_ptr as u64,
		arg1: target_ptr as u64,
		arg2: fstype_ptr as u64,
		context: [0u8; MAX_PATH_LEN],
		context_len: 0,
	};

	unsafe {
		if let Ok(len) = read_str_from_user(&ctx, target_ptr, &mut event.context) {
			event.context_len = len as u32;
		}
	}

	if let Some(mut buf) = EVENTS.reserve::<SandboxEscapeEvent>(0) {
		unsafe {
			buf.as_mut_ptr().write(event);
		}
		buf.submit(0);
	}

	info!(&ctx, "mount: pid={}", header.pid);
	Ok(())
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
	loop {}
}
