extern crate libc;
use libc::{fopen,fwrite,fread,fseek,fflush,c_void,c_char,c_longlong,SEEK_CUR,SEEK_SET};
use std::ffi::{CString};
use std::io::{self, Write};
use std::vec::Vec;

extern { fn puts(s: *const c_char); }


const BLOCK_SIZE: i64 = 0x400;  			//blocksize
const INODE_PER_PAGE: i64 = 2016;		 	//inode per page
const GROUP_DESCRIBERS_SIZE: i64 = 0x20;	//size of group describers

const ADDR:[i32;3] = [0x100,0x10000,0x1000000];

fn read_command()->String{
	let mut raw_command = String::new();
		io::stdin().read_line(&mut raw_command)
			.expect("Failed to read line");
	raw_command
}
fn erase_endl(raw_command:& String)->String{
	let mut temp = String::new();
	let mut count = 0;
	for i in raw_command.chars(){
		count+=1;
		if count!=raw_command.len() {temp.push(i);} 
	}
	temp
}
fn split(command:& String)->Vec<String> {
	let len = command.len();
	let mut temp = String::new();
	let mut cmds_byte = Vec::new();
	let mut cmds_char = Vec::new();
	let mut cmds_all = Vec::new();
	for i in command.as_bytes() { cmds_byte.push(i);}
	for i in command.chars() { cmds_char.push(i); }
	for i in 0..len{
		if cmds_byte[i]==&32u8{
			if temp.len()!=0{cmds_all.push(temp);temp = String::new();}
		}else if cmds_byte[i]==&10u8 {
			if temp.len()!=0{cmds_all.push(temp);temp = String::new();}
		}else {
			temp.push(cmds_char[i]);
		}
	}
	if temp.len()!=0{cmds_all.push(temp);}
	cmds_all
}

// #[derive(Debug)]
struct Dir {
	inode: i32,			//Inode number
	rec_len: i32,		//Directory entry length 
	file_type: i32,		// File Type
	name_len: i32,		//Name length
	name: [c_char;256],	//Name
}
impl Dir {
	fn new(file:*mut libc::FILE)-> Dir{
		let mut result=Dir{inode:0,rec_len:0,file_type:0,name_len:0,name:[0;256]};
		let mut tmp:[c_char;4]=[0,0,0,0];
	    unsafe{fread(tmp.as_ptr() as *mut c_void,4,1,file);}
	    result.inode = toint(tmp.as_mut_ptr(),0,3) as i32;
	    unsafe{fread(tmp.as_ptr() as *mut c_void,2,1,file);}
	    result.rec_len = toint(tmp.as_mut_ptr(),0,1) as i32;
	    unsafe{fread(tmp.as_ptr() as *mut c_void,1,1,file);}
	    result.name_len = tmp[0] as i32;
	    unsafe{fread(tmp.as_ptr() as *mut c_void,1,1,file);}
	    result.file_type = tmp[0] as i32;
	    unsafe{fread(result.name.as_ptr() as *mut c_void,result.name_len as usize,1,file);}
		result
	}
	fn copy(&self)->Dir{
		Dir{inode:self.inode,
			rec_len:self.rec_len,
			file_type:self.file_type,
			name_len:self.name_len,
			name:[0;256]}
	}
}



fn toint(a:*mut i8,s:isize,off:isize)->c_longlong{
	let mut result:i64 = 0;
	if off==1{
		result=((unsafe{(*(a.offset(s+off)))} as i64 )<<8) + 
		(unsafe{(*(a.offset(s+off-1)))} as i64 );
		if unsafe{(*(a.offset(s+off-1)))} <0{
			result -= 0xffffffffffffff00;
		}
		if unsafe{(*(a.offset(s+off)))} <0{
			result -= 0xffffffffffff0000;
		}
	}
	if off==3{
		result = (((unsafe{(*(a.offset(s+off)))} as i32 )<<24) +
		((unsafe{(*(a.offset(s+off-1)))} as i32 )<<16) +
		((unsafe{(*(a.offset(s+off-2)))} as i32 )<<8) +
		(unsafe{(*(a.offset(s+off-3)))} as i32 ) ) as i64;
		if unsafe{(*(a.offset(s+off-3)))} <0{
			result -= 0xffffffffffffff00;
		}
		if unsafe{(*(a.offset(s+off-2)))} <0{
			result -= 0xffffffffffff0000;
		}
		if unsafe{(*(a.offset(s+off-1)))} <0{
			result -= 0xffffffffff000000;
		}
	}
	result
}

fn inode_offset(inode:i64,file:*mut libc::FILE)->i64{
	let num:i64 = inode/INODE_PER_PAGE;
	let result;
	unsafe{
	fseek(file, BLOCK_SIZE*2,SEEK_SET);
	fseek(file, GROUP_DESCRIBERS_SIZE*num,SEEK_CUR);
	let mut group_describer:[c_char;GROUP_DESCRIBERS_SIZE as usize] = [0;GROUP_DESCRIBERS_SIZE as usize];
	fread(group_describer.as_ptr()as *mut c_void,GROUP_DESCRIBERS_SIZE as usize,1,file);
	let position = toint(group_describer.as_mut_ptr(),8,1);
	result = BLOCK_SIZE*position+128*(inode-num*INODE_PER_PAGE-1);
	}
	result
}

fn find_address(blocks:*mut i64,s:i64,x:i32,file:*mut libc::FILE)->isize{
	// unsafe{*blocks.offset(2)=5};
	let mut count = 0;
	if s!=0 {unsafe{fseek(file,BLOCK_SIZE*s,SEEK_SET);}}
	for _ in 0..x {
		let mut tmp:[c_char;4]=[0,0,0,0];
	    unsafe{fread(tmp.as_ptr() as *mut c_void,4,1,file);}
	    // println!("tmp = {:?}", tmp);
	    let tmp_int = toint(tmp.as_mut_ptr(),0,3);
	    // println!("tmp_int[{}] = {:X}",i,tmp_int );
	    if tmp_int==0{break;}
	    unsafe{*blocks.offset(count)=tmp_int;}
	    count+=1;
	}
	count
}

fn itob(blocks:*mut i64,inode:i64,file:*mut libc::FILE)->isize{ //inode to blocks
	let offset = inode_offset(inode,file);
	// println!("offset#: {:?}", offset);
	unsafe{fseek(file,offset+40,SEEK_SET);}
	let mut count = find_address(blocks,0,12,file);
	let mut sec_addr:[i64;3] = [0,0,0];//secondary addressing
	if count==12{
		for i in 0..3{
			let mut tmp:[c_char;4]=[0,0,0,0];
		    unsafe{fread(tmp.as_ptr() as *mut c_void,4,1,file);}
		    sec_addr[i] = toint(tmp.as_mut_ptr(),0,3);
		}
		for i in 0..3{
			if sec_addr[i]!=0 {
				unsafe{
				count+=find_address(blocks.offset(count),sec_addr[i],ADDR[i],file);
				}
			}
		}
	}
	// println!("blocks#: {:?}", unsafe{*blocks});
	count
}
// print out the Dir informations.
fn dir (block:i64,file:*mut libc::FILE,mode:bool){
	unsafe{fseek(file,block*BLOCK_SIZE,SEEK_SET);}
	print!("block: {:X}\n", block); io::stdout().flush().unwrap();
	loop {
		let tmpdir = Dir::new(file);
		if tmpdir.inode!=0{
			print!(" {: >11}: |{: ^9}|{: ^11}|{:^11}|{: ^3}", tmpdir.inode, tmpdir.rec_len,tmpdir.name_len,tmpdir.file_type,"");
			io::stdout().flush().unwrap();
			unsafe{puts(tmpdir.name.as_ptr());}
		}
		if tmpdir.rec_len>=256{break;}
		let mut seek=0;
		if mode==true {seek = tmpdir.rec_len-8-tmpdir.name_len;}
		else {if tmpdir.name_len%4!=0 || tmpdir.name_len==4{seek = 4-tmpdir.name_len%4;}}

		unsafe{fseek(file,seek as i64,SEEK_CUR);}
	}
}
fn find_prev(block:i64,file:*mut libc::FILE,inode:i64){
	unsafe{fseek(file,block*BLOCK_SIZE,SEEK_SET);}
	let mut dir_prev=Dir{inode:0,rec_len:0,file_type:0,name_len:0,name:[0;256]};
	let mut dir_curr=Dir{inode:0,rec_len:0,file_type:0,name_len:0,name:[0;256]};
	loop {
		let tmpdir = Dir::new(file);
		// println!("{:?}", tmpdir.inode);
		dir_curr=tmpdir.copy();
		if tmpdir.inode==inode as i32{break;}
		if tmpdir.rec_len>=256{break;}
		let mut seek=0;
		seek = tmpdir.rec_len-8-tmpdir.name_len;
		unsafe{fseek(file,seek as i64,SEEK_CUR);}
		dir_prev=tmpdir.copy();
	}
	//get the previous inode
	unsafe{fseek(file,block*BLOCK_SIZE,SEEK_SET);}
	loop {
		let tmpdir = Dir::new(file);
		if tmpdir.rec_len>=256{break;}
		let mut seek=0;
		seek = tmpdir.rec_len-8-tmpdir.name_len;
		unsafe{fseek(file,seek as i64,SEEK_CUR);}
		if tmpdir.inode==dir_prev.inode{
			unsafe{fseek(file,(4-tmpdir.rec_len) as i64,SEEK_CUR);}
			break;
		}
	}
	let mut tmp:[c_char;4]=[0,0,0,0];
	unsafe{fread(tmp.as_ptr() as *mut c_void,2,1,file);}
	// println!("P={:?} C={}", dir_prev.rec_len,dir_curr.rec_len);
	let rec_len = toint(tmp.as_mut_ptr(),0,1) as i32;
	let new_rec_len:[c_char;2] =[(dir_prev.rec_len+dir_curr.rec_len) as i8,0]; 
	unsafe{fseek(file,-2 as i64,SEEK_CUR);}
	unsafe{fwrite(new_rec_len.as_ptr() as *const c_void,2,1,file);}
	unsafe{fflush(file);}
}

// ls what's in the target inode
fn ls(inode:i64,file:*mut libc::FILE,mmode:&String){
	println!(" inode number | rec_len |  name_len | file_type | name");
	println!("======================================================================");
	let mut blocks:[c_longlong;30]=[0;30];
	let count = itob(blocks.as_mut_ptr(),inode,file);
	let mut mode = true;
	if mmode == "all"{mode = false;}
	for i in 0..count{
		dir(blocks[i as usize],file,mode);
	}
}
//to cat what's in the file.
fn cat(inode:i64,file:*mut libc::FILE){
	let mut blocks:[c_longlong;300]=[0;300];
	let count = itob(blocks.as_mut_ptr(),inode,file);
	// println!("count = {:?}", count);
	for i in 0..count{
		let buffer:[c_char;BLOCK_SIZE as usize]=[0;BLOCK_SIZE as usize];
		if i==0{ unsafe{fseek(file,blocks[0]*BLOCK_SIZE,SEEK_SET);} }
		else{ unsafe{fseek(file,(blocks[i as usize]-blocks[i as usize -1]-1)*BLOCK_SIZE,SEEK_CUR);} }
		unsafe{fread(buffer.as_ptr() as *mut c_void,BLOCK_SIZE as usize,1,file);}
		unsafe{puts(buffer.as_ptr());} //print content
	}
}
//to backup the file to somewhere else.
fn backup(inode:i64,file:*mut libc::FILE,name:&String){
	let mut blocks:[c_longlong;300]=[0;300];
	let count = itob(blocks.as_mut_ptr(),inode,file);
	let backup;
	unsafe{backup=fopen(CString::new(name.to_string()).unwrap().as_ptr(),CString::new("wb").unwrap().as_ptr());}
	for i in 0..count{
		let buffer:[c_char;BLOCK_SIZE as usize]=[0;BLOCK_SIZE as usize];
		if i==0{ unsafe{fseek(file,blocks[0]*BLOCK_SIZE,SEEK_SET);} }
		else{ unsafe{fseek(file,(blocks[i as usize]-blocks[i as usize -1]-1)*BLOCK_SIZE,SEEK_CUR);} }
		unsafe{fread(buffer.as_ptr() as *mut c_void,BLOCK_SIZE as usize,1,file);}
		unsafe{fwrite(buffer.as_ptr() as *const c_void,BLOCK_SIZE as usize,1,backup);}
		unsafe{fflush(backup);}
	}
}
fn del(inode:i64,file:*mut libc::FILE){
	let mut blocks:[c_longlong;30]=[0;30];
	let count = itob(blocks.as_mut_ptr(),2 as i64,file);
	for i in 0..count{
		find_prev(blocks[i as usize],file,inode);
	}
}

//string to inode_int
fn ctoi(my_string:&String)->i64{
	let my_int: i64 = my_string.parse().unwrap();
	my_int
}

fn main() {
    let mut raw_command;
    loop {
    	raw_command = read_command();
    	if raw_command.len()==0{break;}
    	raw_command = erase_endl(& raw_command);
    	if raw_command.len()==0{continue;}
    	if raw_command == "exit"{break;}
    	let commands = split(& raw_command);
		let file;
		unsafe{file = fopen(CString::new("bean3").unwrap().as_ptr(),CString::new("rw+").unwrap().as_ptr());}
		if commands[0]=="ls"{		
			if commands.len()==3{ls(ctoi(&commands[1]),file,&commands[2]);}
			if commands.len()==2{ls(ctoi(&commands[1]),file,&String::new());}

		}
		if commands[0]=="cat"{		cat(ctoi(&commands[1]),file);}
		if commands[0]=="backup"{	backup(ctoi(&commands[1]),file,&commands[2]);}
		if commands[0]=="del"{	del(ctoi(&commands[1]),file);}
		
    }
}

