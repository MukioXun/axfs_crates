use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use axerrno::LinuxError;
use alloc::ffi::CString;
use core::ffi::{c_int, c_long, c_ulong};

/// Filesystem attributes.
///
/// Currently not used.
#[non_exhaustive]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FileSystemInfo {
    /// 文件系统的类型（magic number，用于标识如 ext4, tmpfs 等）
    pub ftype: u64,
    /// 最优传输块大小（用于文件系统的 I/O 操作优化）
    pub bsize: u64,
    /// 文件系统中数据块的总数量
    pub blocks: u64,
    /// 当前空闲的数据块数量（包括超级用户可用）
    pub bfree: u64,
    /// 普通用户可用的数据块数量（不包括超级用户保留）
    pub bavail: u64,
    /// 文件结点（i-node）总数，表示最多可创建的文件数量
    pub files: u64,
    /// 可用的文件结点数
    pub ffree: u64,
    /// 文件系统标识符（通常是一个唯一的 ID，用于区分不同的文件系统挂载点）
    pub fsid: u64,
    /// 支持的最大文件名长度（单位：字节）
    pub namelen: u64,
}

/// Node (file/directory) attributes.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct VfsNodeAttr {
    dev: u64,
    /// File permission mode.
    mode: VfsNodePerm,
    /// File type.
    ty: VfsNodeType,
    /// Total size, in bytes.
    size: u64,
    /// Number of 512B blocks allocated.
    blocks: u64,
    
    st_ino: u32,
    nlink: u32,
    uid: u16,
    gid: u16,
    nblk_lo: u32,

    atime:u32,
    ctime:u32,
    mtime:u32,

    atime_nse:u32,
    ctime_nse:u32,
    mtime_nse:u32,
}

bitflags::bitflags! {
    /// Node (file/directory) permission mode.
    #[derive(Debug, Clone, Copy)]
    pub struct VfsNodePerm: u16 {
        /// Owner has read permission.
        const OWNER_READ = 0o400;
        /// Owner has write permission.
        const OWNER_WRITE = 0o200;
        /// Owner has execute permission.
        const OWNER_EXEC = 0o100;

        /// Group has read permission.
        const GROUP_READ = 0o40;
        /// Group has write permission.
        const GROUP_WRITE = 0o20;
        /// Group has execute permission.
        const GROUP_EXEC = 0o10;

        /// Others have read permission.
        const OTHER_READ = 0o4;
        /// Others have write permission.
        const OTHER_WRITE = 0o2;
        /// Others have execute permission.
        const OTHER_EXEC = 0o1;
    }
}

/// Node (file/directory) type.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum VfsNodeType {
    /// FIFO (named pipe)
    Fifo = 0o1,
    /// Character device
    CharDevice = 0o2,
    /// Directory
    Dir = 0o4,
    /// Block device
    BlockDevice = 0o6,
    /// Regular file
    File = 0o10,
    /// Symbolic link
    SymLink = 0o12,
    /// Socket
    Socket = 0o14,
}

/// Directory entry.
pub struct VfsDirEntry {
    d_type: VfsNodeType,
    d_name: [u8; 63],
}

impl VfsNodePerm {
    /// Returns the default permission for a file.
    ///
    /// The default permission is `0o666` (owner/group/others can read and write).
    pub const fn default_file() -> Self {
        Self::from_bits_truncate(0o666)
    }

    /// Returns the default permission for a directory.
    ///
    /// The default permission is `0o755` (owner can read, write and execute,
    /// group/others can read and execute).
    pub const fn default_dir() -> Self {
        Self::from_bits_truncate(0o755)
    }

    /// Returns the underlying raw `st_mode` bits that contain the standard
    /// Unix permissions for this file.
    pub const fn mode(&self) -> u32 {
        self.bits() as u32
    }

    /// Returns a 9-bytes string representation of the permission.
    ///
    /// For example, `0o755` is represented as `rwxr-xr-x`.
    pub const fn rwx_buf(&self) -> [u8; 9] {
        let mut perm = [b'-'; 9];
        if self.contains(Self::OWNER_READ) {
            perm[0] = b'r';
        }
        if self.contains(Self::OWNER_WRITE) {
            perm[1] = b'w';
        }
        if self.contains(Self::OWNER_EXEC) {
            perm[2] = b'x';
        }
        if self.contains(Self::GROUP_READ) {
            perm[3] = b'r';
        }
        if self.contains(Self::GROUP_WRITE) {
            perm[4] = b'w';
        }
        if self.contains(Self::GROUP_EXEC) {
            perm[5] = b'x';
        }
        if self.contains(Self::OTHER_READ) {
            perm[6] = b'r';
        }
        if self.contains(Self::OTHER_WRITE) {
            perm[7] = b'w';
        }
        if self.contains(Self::OTHER_EXEC) {
            perm[8] = b'x';
        }
        perm
    }

    /// Whether the owner has read permission.
    pub const fn owner_readable(&self) -> bool {
        self.contains(Self::OWNER_READ)
    }

    /// Whether the owner has write permission.
    pub const fn owner_writable(&self) -> bool {
        self.contains(Self::OWNER_WRITE)
    }

    /// Whether the owner has execute permission.
    pub const fn owner_executable(&self) -> bool {
        self.contains(Self::OWNER_EXEC)
    }
}

impl VfsNodeType {
    /// Tests whether this node type represents a regular file.
    pub const fn is_file(self) -> bool {
        matches!(self, Self::File)
    }

    /// Tests whether this node type represents a directory.
    pub const fn is_dir(self) -> bool {
        matches!(self, Self::Dir)
    }

    /// Tests whether this node type represents a symbolic link.
    pub const fn is_symlink(self) -> bool {
        matches!(self, Self::SymLink)
    }

    /// Returns `true` if this node type is a block device.
    pub const fn is_block_device(self) -> bool {
        matches!(self, Self::BlockDevice)
    }

    /// Returns `true` if this node type is a char device.
    pub const fn is_char_device(self) -> bool {
        matches!(self, Self::CharDevice)
    }

    /// Returns `true` if this node type is a fifo.
    pub const fn is_fifo(self) -> bool {
        matches!(self, Self::Fifo)
    }

    /// Returns `true` if this node type is a socket.
    pub const fn is_socket(self) -> bool {
        matches!(self, Self::Socket)
    }

    /// Returns a character representation of the node type.
    ///
    /// For example, `d` for directory, `-` for regular file, etc.
    pub const fn as_char(self) -> char {
        match self {
            Self::Fifo => 'p',
            Self::CharDevice => 'c',
            Self::Dir => 'd',
            Self::BlockDevice => 'b',
            Self::File => '-',
            Self::SymLink => 'l',
            Self::Socket => 's',
        }
    }
}

impl VfsNodeAttr {
    /// Creates a new `VfsNodeAttr` with the given permission mode, type, size
    /// and number of blocks.
    pub const fn new(dev: u64, mode: VfsNodePerm, ty: VfsNodeType, size: u64, blocks: u64, st_ino: u32, nlink: u32, uid: u16, gid: u16, nblk_lo: u32, atime:u32, ctime:u32, mtime:u32, atime_nsec:u32, mtime_nsec:u32, ctime_nsec:u32) -> Self {
        Self {
            dev,
            mode,
            ty,
            size,
            blocks,
            st_ino,
            nlink,
            uid,
            gid,
            nblk_lo,
            atime,
            ctime,
            mtime,
            atime_nse:atime_nsec,
            ctime_nse:ctime_nsec,
            mtime_nse:mtime_nsec,
        }
    }

    /// Creates a new `VfsNodeAttr` for a file, with the default file permission.
    pub const fn new_file(size: u64, blocks: u64) -> Self {
        Self {
            dev: 0,
            mode: VfsNodePerm::default_file(),
            ty: VfsNodeType::File,
            size,
            blocks,
            st_ino:0,
            nlink:0,
            uid:0,
            gid:0,
            nblk_lo:0,
            atime:0,
            ctime:0,
            mtime:0,
            atime_nse:0,
            ctime_nse:0,
            mtime_nse:0,
        }
    }

    /// Creates a new `VfsNodeAttr` for a directory, with the default directory
    /// permission.
    pub const fn new_dir(size: u64, blocks: u64) -> Self {
        Self {
            dev: 0,
            mode: VfsNodePerm::default_dir(),
            ty: VfsNodeType::Dir,
            size,
            blocks,
            st_ino:0,
            nlink:0,
            uid:0,
            gid:0,
            nblk_lo:0,
            atime:0,
            ctime:0,
            mtime:0,
            atime_nse:0,
            ctime_nse:0,
            mtime_nse:0,
        }
    }

    /// Returns the size of the node.
    pub const fn size(&self) -> u64 {
        self.size
    }

    /// Returns the number of blocks the node occupies on the disk.
    pub const fn blocks(&self) -> u64 {
        self.blocks
    }

    /// Returns the permission of the node.
    pub const fn perm(&self) -> VfsNodePerm {
        self.mode
    }

    /// Sets the permission of the node.
    pub fn set_perm(&mut self, perm: VfsNodePerm) {
        self.mode = perm
    }

    /// Returns the type of the node.
    pub const fn file_type(&self) -> VfsNodeType {
        self.ty
    }

    /// Whether the node is a file.
    pub const fn is_file(&self) -> bool {
        self.ty.is_file()
    }

    /// Whether the node is a directory.
    pub const fn is_dir(&self) -> bool {
        self.ty.is_dir()
    }
    
    pub const fn st_ino(&self) -> u32 {self.st_ino}
    pub const fn nlink(&self) -> u32 {self.nlink}
    pub const fn uid(&self) -> u16 {self.uid}
    pub const fn gid(&self) -> u16 {self.gid}
    pub const fn nblk_lo(&self) -> u32 {self.nblk_lo}

    pub const fn atime(&self) -> u32{self.atime}
    pub const fn mtime(&self) -> u32 {self.mtime}
    pub const fn ctime(&self) -> u32{self.ctime}
    
    pub const fn mtime_nse(&self) -> u32 {self.mtime_nse}
    pub const fn atime_nse(&self) -> u32 {self.atime_nse}
    pub const fn ctime_nse(&self) -> u32 {self.ctime_nse}
    pub const fn dev(&self) -> u64 {self.dev}
}

impl VfsDirEntry {
    /// Creates an empty `VfsDirEntry`.
    pub const fn default() -> Self {
        Self {
            d_type: VfsNodeType::File,
            d_name: [0; 63],
        }
    }

    /// Creates a new `VfsDirEntry` with the given name and type.
    pub fn new(name: &str, ty: VfsNodeType) -> Self {
        let mut d_name = [0; 63];
        if name.len() > d_name.len() {
            log::warn!(
                "directory entry name too long: {} > {}",
                name.len(),
                d_name.len()
            );
        }
        d_name[..name.len()].copy_from_slice(name.as_bytes());
        Self { d_type: ty, d_name }
    }

    /// Returns the type of the entry.
    pub fn entry_type(&self) -> VfsNodeType {
        self.d_type
    }

    /// Converts the name of the entry to a byte slice.
    pub fn name_as_bytes(&self) -> &[u8] {
        let len = self
            .d_name
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(self.d_name.len());
        &self.d_name[..len]
    }
}
