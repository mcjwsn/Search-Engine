use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;
use std::time::Instant;
use crate::{Document, PreprocessedData, SerMatrix, SerializableCsrMatrix, SvdData};

pub fn load_svd_data(filepath: &str) -> Result<SvdData, Box<dyn Error>> {
    println!("Loading SVD data from {}...", filepath);
    let start_total = Instant::now();

    let index_file = File::open(filepath)?;
    let reader = BufReader::new(index_file);
    let (meta_path, u_path, vt_path, docs_path): (String, String, String, String) =
        bincode::deserialize_from(reader)?;

    println!("Found component files in index.");

    println!("Loading SVD metadata from {}...", meta_path);
    let meta_start = Instant::now();
    let meta_file = File::open(&meta_path)?;
    let meta_reader = BufReader::new(meta_file);
    let (rank, sigma_k): (usize, Vec<f64>) = bincode::deserialize_from(meta_reader)?;
    println!("Metadata loaded in {:?}", meta_start.elapsed());

    println!("Loading U matrix from {}...", u_path);
    let u_start = Instant::now();

    let mut u_file = File::open(&u_path)?;
    let u_file_size = u_file.metadata()?.len() as usize;
    println!("U matrix file size: {} bytes", u_file_size);

    let mut u_reader = BufReader::with_capacity(8 * 1024 * 1024, u_file);

    let u_nrows: usize = match bincode::deserialize_from(&mut u_reader) {
        Ok(val) => val,
        Err(e) => {
            println!("Error reading U matrix nrows: {}", e);
            return Err(Box::new(e));
        }
    };

    let u_ncols: usize = match bincode::deserialize_from(&mut u_reader) {
        Ok(val) => val,
        Err(e) => {
            println!("Error reading U matrix ncols: {}", e);
            return Err(Box::new(e));
        }
    };

    println!("U matrix dimensions: {}x{}", u_nrows, u_ncols);

    let u_total_size = u_nrows * u_ncols;
    let expected_data_bytes = u_total_size * size_of::<f64>();
    println!("Expected U matrix data size: {} elements ({} bytes)", u_total_size, expected_data_bytes);

    let mut u_data = Vec::with_capacity(u_total_size);

    let result: Result<Vec<f64>, _> = bincode::deserialize_from(&mut u_reader);

    match result {
        Ok(data) => {
            println!("Successfully read U matrix data: {} elements", data.len());
            u_data = data;
        },
        Err(e) => {
            println!("Error deserializing U matrix data: {}", e);

            println!("Creating empty U matrix with zeros");
            u_data = vec![0.0; u_total_size];
        }
    }

    if u_data.len() != u_total_size {
        println!("Warning: U matrix data size mismatch. Expected: {}, Found: {}",
                 u_total_size, u_data.len());

        if u_data.len() < u_total_size {
            println!("Padding with zeros to reach correct size");
            u_data.resize(u_total_size, 0.0);
        } else {
            println!("Truncating to correct size");
            u_data.truncate(u_total_size);
        }
    }

    let u_ser = SerMatrix {
        nrows: u_nrows,
        ncols: u_ncols,
        data: u_data,
    };
    println!("U matrix loaded in {:?}", u_start.elapsed());

    println!("Loading V^T matrix from {}...", vt_path);
    let vt_start = Instant::now();

    let vt_file = File::open(&vt_path)?;
    let vt_file_size = vt_file.metadata()?.len() as usize;
    println!("V^T matrix file size: {} bytes", vt_file_size);

    let mut vt_reader = BufReader::with_capacity(8 * 1024 * 1024, vt_file);

    let vt_nrows: usize = match bincode::deserialize_from(&mut vt_reader) {
        Ok(val) => val,
        Err(e) => {
            println!("Error reading V^T matrix nrows: {}", e);
            return Err(Box::new(e));
        }
    };

    let vt_ncols: usize = match bincode::deserialize_from(&mut vt_reader) {
        Ok(val) => val,
        Err(e) => {
            println!("Error reading V^T matrix ncols: {}", e);
            return Err(Box::new(e));
        }
    };

    println!("V^T matrix dimensions: {}x{}", vt_nrows, vt_ncols);

    let vt_total_size = vt_nrows * vt_ncols;
    let expected_vt_bytes = vt_total_size * std::mem::size_of::<f64>();
    println!("Expected V^T matrix data size: {} elements ({} bytes)", vt_total_size, expected_vt_bytes);

    let mut vt_data = Vec::with_capacity(vt_total_size);
    let vt_result: Result<Vec<f64>, _> = bincode::deserialize_from(&mut vt_reader);

    match vt_result {
        Ok(data) => {
            println!("Successfully read V^T matrix data: {} elements", data.len());
            vt_data = data;
        },
        Err(e) => {
            println!("Error deserializing V^T matrix data: {}", e);

            println!("Creating empty V^T matrix with zeros");
            vt_data = vec![0.0; vt_total_size];
        }
    }

    if vt_data.len() != vt_total_size {
        println!("Warning: V^T matrix data size mismatch. Expected: {}, Found: {}",
                 vt_total_size, vt_data.len());

        if vt_data.len() < vt_total_size {
            println!("Padding with zeros to reach correct size");
            vt_data.resize(vt_total_size, 0.0);
        } else {
            println!("Truncating to correct size");
            vt_data.truncate(vt_total_size);
        }
    }

    let vt_ser = SerMatrix {
        nrows: vt_nrows,
        ncols: vt_ncols,
        data: vt_data,
    };
    println!("V^T matrix loaded in {:?}", vt_start.elapsed());

    println!("Loading document vectors from {}...", docs_path);
    let docs_start = Instant::now();

    let docs_file = File::open(&docs_path)?;
    let docs_file_size = docs_file.metadata()?.len() as usize;
    println!("Document vectors file size: {} bytes", docs_file_size);

    let mut docs_reader = BufReader::with_capacity(8 * 1024 * 1024, docs_file);

    let docs_nrows: usize = match bincode::deserialize_from(&mut docs_reader) {
        Ok(val) => val,
        Err(e) => {
            println!("Error reading document vectors nrows: {}", e);
            return Err(Box::new(e));
        }
    };

    let docs_ncols: usize = match bincode::deserialize_from(&mut docs_reader) {
        Ok(val) => val,
        Err(e) => {
            println!("Error reading document vectors ncols: {}", e);
            return Err(Box::new(e));
        }
    };

    println!("Document vectors dimensions: {}x{}", docs_nrows, docs_ncols);

    let docs_total_size = docs_nrows * docs_ncols;
    let expected_docs_bytes = docs_total_size * std::mem::size_of::<f64>();
    println!("Expected document vectors data size: {} elements ({} bytes)", docs_total_size, expected_docs_bytes);

    let mut docs_data = Vec::with_capacity(docs_total_size);
    let docs_result: Result<Vec<f64>, _> = bincode::deserialize_from(&mut docs_reader);

    match docs_result {
        Ok(data) => {
            println!("Successfully read document vectors data: {} elements", data.len());
            docs_data = data;
        },
        Err(e) => {
            println!("Error deserializing document vectors data: {}", e);

            println!("Creating empty document vectors with zeros");
            docs_data = vec![0.0; docs_total_size];
        }
    }

    if docs_data.len() != docs_total_size {
        println!("Warning: Document vectors data size mismatch. Expected: {}, Found: {}",
                 docs_total_size, docs_data.len());

        if docs_data.len() < docs_total_size {
            println!("Padding with zeros to reach correct size");
            docs_data.resize(docs_total_size, 0.0);
        } else {
            println!("Truncating to correct size");
            docs_data.truncate(docs_total_size);
        }
    }

    let docs_ser = SerMatrix {
        nrows: docs_nrows,
        ncols: docs_ncols,
        data: docs_data,
    };
    println!("Document vectors loaded in {:?}", docs_start.elapsed());

    let svd_data = SvdData {
        rank,
        sigma_k,
        u_ser,
        vt_ser,
        docs_ser,
    };

    println!("All SVD data loaded successfully in {:?}!", start_total.elapsed());
    Ok(svd_data)
}

pub fn load_preprocessed_data(filepath: &str) -> Result<PreprocessedData, Box<dyn Error>> {
    println!("Loading preprocessed data from {}...", filepath);
    let start_total = Instant::now();

    let index_file = File::open(filepath)?;
    let reader = BufReader::with_capacity(1024 * 1024, index_file); // 1MB buffer
    let (dict_path, docs_path, matrix_path): (String, String, String) =
        bincode::deserialize_from(reader)?;
    println!("Found component files in index.");

    println!("Loading term dictionary from {}...", dict_path);
    let dict_start = Instant::now();
    let dict_file = File::open(dict_path)?;
    let dict_reader = BufReader::with_capacity(1024 * 1024, dict_file);
    let (term_dict, inverse_term_dict, idf): (
        HashMap<String, usize>,
        HashMap<usize, String>,
        Vec<f64>
    ) = bincode::deserialize_from(dict_reader)?;
    println!("Dictionary loaded in {:?}", dict_start.elapsed());

    println!("Loading documents from {}...", docs_path);
    let docs_start = Instant::now();
    let docs_file = File::open(docs_path)?;
    let docs_reader = BufReader::with_capacity(1024 * 1024, docs_file);
    let documents: Vec<Document> = bincode::deserialize_from(docs_reader)?;
    println!("Documents loaded in {:?}", docs_start.elapsed());

    println!("Loading term-document matrix from {}...", matrix_path);
    let matrix_start = Instant::now();
    let matrix_file = File::open(matrix_path)?;
    let mut buffer = BufReader::with_capacity(8 * 1024 * 1024, matrix_file); // 8MB buffer dla większej macierzy

    let nrows: usize = bincode::deserialize_from(&mut buffer)?;
    let ncols: usize = bincode::deserialize_from(&mut buffer)?;


    let row_offsets: Vec<usize> = bincode::deserialize_from(&mut buffer)?;

    let col_indices: Vec<usize> = bincode::deserialize_from(&mut buffer)?;
    let values: Vec<f64> = bincode::deserialize_from(&mut buffer)?;
    println!("Matrix loaded in {:?}", matrix_start.elapsed());

    let term_doc_csr = SerializableCsrMatrix {
        nrows,
        ncols,
        row_offsets,
        col_indices,
        values,
    };

    let preprocessed_data = PreprocessedData {
        term_dict,
        inverse_term_dict,
        idf,
        documents,
        term_doc_csr,
    };

    println!("All data loaded successfully in {:?}!", start_total.elapsed());
    Ok(preprocessed_data)
}

pub fn save_svd_data(
    data: &SvdData,
    filepath: &str,
) -> Result<(), Box<dyn Error>> {
    println!("Saving SVD data to {}...", filepath);
    let start_total = Instant::now();

    let base_path = Path::new(filepath).with_extension("");
    let base_path_str = base_path.to_string_lossy();

    let meta_path = format!("{}_meta.bin", base_path_str);
    println!("Saving SVD metadata to {}...", meta_path);
    let meta_start = Instant::now();
    let meta_file = File::create(&meta_path)?;
    let meta_data = (data.rank, &data.sigma_k);
    bincode::serialize_into(meta_file, &meta_data)?;
    println!("Metadata saved in {:?}", meta_start.elapsed());

    let u_path = format!("{}_u.bin", base_path_str);
    println!("Saving U matrix ({}x{}) to {}...",
             data.u_ser.nrows, data.u_ser.ncols, u_path);
    let u_start = Instant::now();
    let u_file = File::create(&u_path)?;
    let mut u_buffer = BufWriter::with_capacity(4 * 1024 * 1024, u_file);

    bincode::serialize_into(&mut u_buffer, &data.u_ser.nrows)?;
    bincode::serialize_into(&mut u_buffer, &data.u_ser.ncols)?;

    const CHUNK_SIZE: usize = 1_000_000;
    let u_data = &data.u_ser.data;
    let mut i = 0;
    while i < u_data.len() {
        let end = (i + CHUNK_SIZE).min(u_data.len());
        let chunk = &u_data[i..end];
        bincode::serialize_into(&mut u_buffer, &chunk)?;
        i = end;
    }
    u_buffer.flush()?;
    println!("U matrix saved in {:?}", u_start.elapsed());

    let vt_path = format!("{}_vt.bin", base_path_str);
    println!("Saving V^T matrix to {}...", vt_path);
    let vt_start = Instant::now();
    let vt_file = File::create(&vt_path)?;
    let mut vt_buffer = io::BufWriter::with_capacity(4 * 1024 * 1024, vt_file); // 4MB buffer

    bincode::serialize_into(&mut vt_buffer, &data.vt_ser.nrows)?;
    bincode::serialize_into(&mut vt_buffer, &data.vt_ser.ncols)?;

    let vt_data = &data.vt_ser.data;
    let mut i = 0;
    while i < vt_data.len() {
        let end = (i + CHUNK_SIZE).min(vt_data.len());
        let chunk = &vt_data[i..end];
        bincode::serialize_into(&mut vt_buffer, &chunk)?;
        i = end;
    }
    vt_buffer.flush()?;
    println!("V^T matrix saved in {:?}", vt_start.elapsed());

    let docs_path = format!("{}_docs.bin", base_path_str);
    println!("Saving document vectors to {}...", docs_path);
    let docs_start = Instant::now();
    let docs_file = File::create(&docs_path)?;
    let mut docs_buffer = io::BufWriter::with_capacity(4 * 1024 * 1024, docs_file); // 4MB buffer

    bincode::serialize_into(&mut docs_buffer, &data.docs_ser.nrows)?;
    bincode::serialize_into(&mut docs_buffer, &data.docs_ser.ncols)?;

    let docs_data = &data.docs_ser.data;
    let mut i = 0;
    while i < docs_data.len() {
        let end = (i + CHUNK_SIZE).min(docs_data.len());
        let chunk = &docs_data[i..end];
        bincode::serialize_into(&mut docs_buffer, &chunk)?;
        i = end;
    }
    docs_buffer.flush()?;
    println!("Document vectors saved in {:?}", docs_start.elapsed());

    let index_path = filepath;
    println!("Creating index file at {}...", index_path);
    let index_file = File::create(index_path)?;
    let index_data = (
        meta_path,
        u_path,
        vt_path,
        docs_path
    );
    bincode::serialize_into(index_file, &index_data)?;

    println!("All SVD data saved successfully in {:?}!", start_total.elapsed());
    Ok(())
}

pub fn save_preprocessed_data(
    data: &PreprocessedData,
    filepath: &str,
) -> Result<(), Box<dyn Error>> {
    println!("Saving preprocessed data to {}...", filepath);
    let start_total = Instant::now();

    let base_path = Path::new(filepath).with_extension("");
    let base_path_str = base_path.to_string_lossy();

    let dict_path = format!("{}_terms.bin", base_path_str);
    println!("Saving term dictionary to {}...", dict_path);
    let dict_start = Instant::now();
    let dict_file = File::create(&dict_path)?;
    let dict_data = (&data.term_dict, &data.inverse_term_dict, &data.idf);
    bincode::serialize_into(dict_file, &dict_data)?;
    println!("Dictionary saved in {:?}", dict_start.elapsed());

    let docs_path = format!("{}_docs.bin", base_path_str);
    println!("Saving documents to {}...", docs_path);
    let docs_start = Instant::now();
    let docs_file = File::create(&docs_path)?;
    bincode::serialize_into(docs_file, &data.documents)?;
    println!("Documents saved in {:?}", docs_start.elapsed());

    let matrix_path = format!("{}_matrix.bin", base_path_str);
    println!("Saving term-document matrix to {}...", matrix_path);
    let matrix_start = Instant::now();

    let matrix_file = File::create(&matrix_path)?;
    let mut buffer = io::BufWriter::with_capacity(1024 * 1024, matrix_file); // 1MB buffer

    bincode::serialize_into(&mut buffer, &data.term_doc_csr.nrows)?;
    bincode::serialize_into(&mut buffer, &data.term_doc_csr.ncols)?;

    bincode::serialize_into(&mut buffer, &data.term_doc_csr.row_offsets)?;

    bincode::serialize_into(&mut buffer, &data.term_doc_csr.col_indices)?;
    bincode::serialize_into(&mut buffer, &data.term_doc_csr.values)?;

    buffer.flush()?;
    println!("Matrix saved in {:?}", matrix_start.elapsed());

    let index_path = filepath;
    println!("Creating index file at {}...", index_path);
    let index_file = File::create(index_path)?;
    let index_data = (
        dict_path,
        docs_path,
        matrix_path
    );
    bincode::serialize_into(index_file, &index_data)?;

    println!("All data saved successfully in {:?}!", start_total.elapsed());
    Ok(())
}