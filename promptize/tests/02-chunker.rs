use any_chunk::Chunkable;

#[derive(Debug, Chunkable)]
pub struct ChunkableStruct {
    executable: String,
    #[chunkable]
    file_contents: Vec<String>,
    env: Vec<String>,
    current_dir: String
}

// impl ChunkableStruct {
//     fn chunk_it(self) -> Vec<Self> {
//         let chunks: Vec<_> = self.file_contents.chunks(300).collect();
//         
//         let chunks = chunks
//             .into_iter()
//             .map(|c| {
//                 return ChunkableStruct {
//                     executable: self.executable.clone(),
//                     file_contents: c.to_vec(),
//                     env: self.env.clone(),
//                     current_dir: self.current_dir.clone()
//                 }
//             })
//             .collect();
//
//         return chunks;
//     }
// }

fn main() {
    let test_prompt = ChunkableStruct {
        executable: "Foo".to_string(),
        file_contents: vec!["Foo".to_string()],
        env: vec!["Foo".to_string()],
        current_dir: "Foo".to_string(),
    };

    let chunked_prompt = test_prompt.chunk_it();

    println!("{:#?}", chunked_prompt);
}
