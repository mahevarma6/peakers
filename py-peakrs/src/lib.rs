use pyo3::prelude::*;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::fs::{File, metadata};
use std::str;
use std::io::{Write, BufWriter};

#[pyclass]
#[derive(Clone)]
struct CsvMeta {
    total_column: i32,
    validate_row: i64,
    estimate_row: i64,
    is_line_br_13_exist: bool,
    is_line_br_10_exist: bool,
    column_name: Vec<String>,
    file_size: i64,
    delimiter: u8,   
    error_message: String, 
}

#[pymethods]
impl CsvMeta {
    #[getter]
    fn get_total_column(&self) -> PyResult<i32> {
        Ok(self.total_column)
    }

    #[getter]
    fn get_validate_row(&self) -> PyResult<i64> {
        Ok(self.validate_row)
    }

    #[getter]
    fn get_estimate_row(&self) -> PyResult<i64> {
        Ok(self.estimate_row)
    }

    #[getter]
    fn get_is_line_br_13_exist(&self) -> PyResult<bool> {
        Ok(self.is_line_br_13_exist)
    }

    #[getter]
    fn get_is_line_br_10_exist(&self) -> PyResult<bool> {
        Ok(self.is_line_br_10_exist)
    }

    #[getter]
    fn get_column_name(&self) -> PyResult<Vec<String>> {
        Ok(self.column_name.clone())
    }

    #[getter]
    fn get_file_size(&self) -> PyResult<i64> {
        Ok(self.file_size)
    }

    #[getter]
    fn get_delimiter(&self) -> PyResult<u8> {
        Ok(self.delimiter)
    }   

    
    #[getter]
    fn get_error_message(&self) -> PyResult<String> {
        Ok(self.error_message.clone())
    }       
}


fn get_byte_array_frequency_distribution(byte_array: &Vec<u8>) -> HashMap<u8, i32> {
   
    let mut frequency_distribution = HashMap::new();

    for &item in byte_array {
        *frequency_distribution.entry(item).or_insert(0) += 1;
    }

    frequency_distribution
}

fn get_current_row_frequency_distribution(file: &mut File, start_byte: i64) -> (usize, HashMap<u8, i32>, Vec<u8>) {

    let mut frequency_distribution = HashMap::new();
    let mut is_valid_row_exist = false;
    let mut sample_size = 0;
    let mut double_quote_count = 0;
    let mut current_row = Vec::new();

    while !is_valid_row_exist && sample_size < 10000 {
        sample_size += 100;

        let mut byte_array = vec![0; sample_size];
        
        file.seek(SeekFrom::Start(start_byte as u64)).unwrap();
        file.read_exact(&mut byte_array).unwrap();

        let mut n = 0;
        current_row.clear();
        let mut is_first_line_break_exist = false;
        let mut is_second_line_break_exist = false;

        if start_byte == 0 {
            is_first_line_break_exist = true;
        }

        while n < sample_size && !is_second_line_break_exist {
            if !is_first_line_break_exist && byte_array[n] == 10 {
                is_first_line_break_exist = true;
            } else if is_first_line_break_exist && byte_array[n] == 10 {
                is_second_line_break_exist = true;
            }

            if is_first_line_break_exist {
                if current_row.is_empty() && byte_array[n] == 10 {
                } else {
                    if byte_array[n] == 34 {
                        double_quote_count += 1;
                    }

                    if double_quote_count % 2 == 0 {
                        current_row.push(byte_array[n]);
                    }
                }
            }
            n += 1;
        }

        if is_second_line_break_exist && !current_row.is_empty() {
            frequency_distribution =
                get_byte_array_frequency_distribution(&current_row.clone());
            is_valid_row_exist = true;
        }
    }

    (current_row.len(), frequency_distribution, current_row)
}

fn skip_white_space(byte_array: &Vec<u8>, mut start_byte: i64, mut end_byte: i64) -> (i64, i64) {
    
    while start_byte < end_byte && byte_array[start_byte as usize] == 32 {
        start_byte += 1;
    }

    while end_byte > start_byte && byte_array[(end_byte - 1) as usize] == 32 {
        end_byte -= 1;
    }

    (start_byte, end_byte)
}

fn get_column_name(file: &mut File, delimiter: u8) -> Vec<String> {
   
    let mut is_valid_row_exist = false;
    let mut sample_size = 0;
    let mut column_name = Vec::new();

    while !is_valid_row_exist {
        let mut column_count = 0;
        let mut double_quote_count = 0;
        let mut cell_address = Vec::new();
        column_name.clear();

        sample_size += 100;

        let mut byte_array = vec![0; sample_size as usize];
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_exact(&mut byte_array).unwrap();

        let mut n = 0;

        cell_address.push(0);

        while n < sample_size {
            if byte_array[n as usize] == delimiter {
                if double_quote_count % 2 == 0 {
                    cell_address.push(n + 1);
                    column_count += 1;
                }
            } else if byte_array[n as usize] == 10 {
                cell_address.push(n + 1);
                is_valid_row_exist = true;
                column_count += 1;
                break;
            } else if byte_array[n as usize] == 13 {
                cell_address.push(n + 1);
            } else if byte_array[n as usize] == 34 {
                double_quote_count += 1;
            }
            n += 1;
        }

        let mut start_byte;
        let mut end_byte;

        for i in 0..column_count {
            start_byte = cell_address[i];
            end_byte = cell_address[i + 1] - 1;
            let (start_byte, end_byte) =
                skip_white_space(&byte_array, start_byte, end_byte);
            column_name.push(
                String::from_utf8(byte_array[start_byte as usize..end_byte as usize].to_vec())
                    .unwrap(),
            );
        }
    }

    column_name
}

fn cell_address(byte_array: &Vec<u8>, csv_meta: CsvMeta) -> Vec<i64> {
    
    let read_csv_delimiter = csv_meta.delimiter;

    let mut cell_address = Vec::new();
    let mut double_quote_count = 0;

    cell_address.push(0);
    let mut _row = 0;

    for (i, &byte) in byte_array.iter().enumerate() {
        if byte == read_csv_delimiter {
            if double_quote_count != 1 {
                cell_address.push(i as i64 + 1);
                double_quote_count = 0;
            }
        } else if byte == 10 {
            cell_address.push(i as i64 + 1);
            _row += 1;
        } else if byte == 13 {
            cell_address.push(i as i64 + 1);
        } else if byte == 34 {
            double_quote_count += 1;
        }
    }

    cell_address
}

fn current_view(byte_array: &Vec<u8>, csv_meta: CsvMeta, start_column: i32, end_column: i32, total_row: i32) {
    
    let cell_address = cell_address(byte_array, csv_meta.clone());

    let extra_line_br_char = if csv_meta.is_line_br_13_exist { 1 } else { 0 };
    
    let max_col = csv_meta.total_column + extra_line_br_char - 1;    
    let cell = cell_address.len() as u32 - max_col as u32 + 1;
    let mut n = 0u32;
    let mut current_row = 0i32;    
    let mut temp_bytes: Vec<u8> = Vec::new();
    let read_csv_delimiter = csv_meta.delimiter;
    let mut max_text_length = HashMap::new();
    let mut max_integer_length = HashMap::new();
    let mut max_decimal_length = HashMap::new();
    let mut max_column_width = HashMap::new();
    let mut is_column_real_number = HashMap::new();

    let csv_meta = csv_meta.clone();

    for current_column in start_column..end_column {        
        max_text_length.insert(current_column, 1);
        max_integer_length.insert(current_column, 1);
        max_decimal_length.insert(current_column, 1);
        max_column_width.insert(current_column, csv_meta.column_name[current_column as usize].len());
        is_column_real_number.insert(current_column, true);
    }

    while n < cell {

    for current_column in start_column..end_column {

        if current_column > 0 {
            temp_bytes.push(read_csv_delimiter);
        }

        let current_cell = (max_col + 1) as i32 * current_row + current_column as i32;
        let start_byte = cell_address[current_cell as usize];
        let end_byte = cell_address[(current_cell + 1) as usize] - 1;
        let (start_byte, end_byte) = skip_white_space(byte_array, start_byte, end_byte);

        let mut is_real_number = true;
        let mut _is_float = false;
        let mut is_negative_sign_exist = false;
        let mut is_open_bracket = false;
        let mut is_close_bracket = false;
        let mut integer_length = 0;
        let mut decimal_length = 0;
        let mut is_dot_exist = false;
        let mut _text_length = 0;

        for current_byte in start_byte..end_byte {
            match byte_array[current_byte as usize] {
                b'.' => {
                    if _is_float {
                        // exist more than one time
                        _is_float = false;
                        is_real_number = false;
                    }
                    _is_float = true;
                    if current_byte == start_byte || current_byte == end_byte - 1 {
                        is_real_number = false;
                    }
                }
                b'-' => {
                    if is_negative_sign_exist {
                        is_negative_sign_exist = false;
                        is_real_number = false;
                    }
                    if current_byte == start_byte {
                        is_negative_sign_exist = true;
                    } else {
                        is_real_number = false;
                    }
                }
                b'(' => {
                    if current_byte == start_byte {
                        is_open_bracket = true;
                    } else {
                        is_real_number = false;
                    }
                }
                b')' => {
                    if current_byte == end_byte - 1 {
                        is_close_bracket = true;
                    } else {
                        is_real_number = false;
                    }
                }
                _ => {
                    if byte_array[current_byte as usize] < b'0'
                        || byte_array[current_byte as usize] > b'9'
                    {
                        is_real_number = false;
                    }
                }
            }
        }

        if is_negative_sign_exist {
            is_real_number = true;
        }

        if is_open_bracket && is_close_bracket {
            is_real_number = true;
        }

        if is_real_number {
            for current_byte in start_byte..end_byte {
                if byte_array[current_byte as usize] == b'.' {
                    is_dot_exist = true;
                } else if is_dot_exist {
                    decimal_length += 1;
                } else {
                    integer_length += 1;
                }
            }

            if integer_length > max_integer_length[&current_column] {
                max_integer_length.insert(current_column, integer_length);
            }

            if decimal_length > max_decimal_length[&current_column] {
                max_decimal_length.insert(current_column, decimal_length);
            }
        }

        *is_column_real_number.get_mut(&current_column).unwrap() =
            *is_column_real_number.get(&current_column).unwrap() && is_real_number;

        _text_length = (end_byte - start_byte) as i32;

        if _text_length > max_text_length[&current_column] {
            max_text_length.insert(current_column, _text_length);
        }

        temp_bytes.extend_from_slice(&byte_array[start_byte as usize..end_byte as usize]);
    }

    n += (max_col + 1) as u32;
    current_row += 1;

    if current_row >= total_row as i32 {
        break;
    }

    temp_bytes.clear();
}


    for current_column in start_column..end_column {
        let current_length =
            max_integer_length[&current_column] + max_decimal_length[&current_column] + 1;

        if is_column_real_number[&current_column] {
            if max_column_width[&current_column] < current_length {
                max_column_width.insert(current_column, current_length);
            }
        } else {

            let cur_col_max_text_length = max_text_length[&current_column];
            
            if max_column_width[&current_column] < cur_col_max_text_length as usize {                
                max_column_width.insert(current_column, cur_col_max_text_length as usize);
            }
        }
    }

    let mut result_bytes: Vec<u8> = vec![];

    result_bytes.push(32);
    result_bytes.push(32);    

    for current_column in start_column..end_column {
        // Display Column Name
        let current_columm_name = &csv_meta.column_name[current_column as usize];
        let current_length = current_columm_name.len();

        if is_column_real_number[&current_column] {

            if max_column_width[&current_column] > current_length {

                for _ in 0..max_column_width[&current_column] - current_length {
                    result_bytes.push(32);
                }
            }

            result_bytes.extend_from_slice(current_columm_name.as_bytes());

            result_bytes.push(32);
            result_bytes.push(32);

        } else {

            result_bytes.extend_from_slice(current_columm_name.as_bytes());

            if max_column_width[&current_column] > current_length {
                for _ in 0..max_column_width[&current_column] - current_length {
                    result_bytes.push(32);
                }
            }

            result_bytes.push(32);
            result_bytes.push(32);
        }
    }

    result_bytes.push(13);
    result_bytes.push(10);

    n = 0;
    current_row = 0;

    while n < cell {
        result_bytes.push(32);
        result_bytes.push(32);

        for current_column in start_column..end_column {
            let current_cell = (max_col as i32 + 1) * current_row + current_column as i32;
            let start_byte = cell_address[current_cell as usize];
            let end_byte = cell_address[current_cell as usize + 1] - 1;
            let (start_byte, end_byte) = skip_white_space(byte_array, start_byte, end_byte);

            let current_length: i32;

            if is_column_real_number[&current_column] {
                if max_decimal_length[&current_column] == 0 {
                    current_length = end_byte as i32 - start_byte as i32;

                    if max_column_width[&current_column] > current_length as usize {
                        for _ in 0..max_column_width[&current_column] - current_length as usize {
                            result_bytes.push(32);
                        }
                    }

                    for current_byte in start_byte..end_byte {
                        result_bytes.push(byte_array[current_byte as usize]);
                    }

                    result_bytes.push(32);
                    result_bytes.push(32);
                } else {
                    current_length = end_byte as i32 - start_byte as i32;
                    let mut current_integer_length = 0;

                    for current_byte in start_byte..end_byte {
                        if byte_array[current_byte as usize] == 46 {
                            break;
                        }

                        current_integer_length += 1;
                    }

                    let left_space =
                        max_column_width[&current_column] - max_integer_length[&current_column]
                            - max_decimal_length[&current_column]
                            - 1;

                    for _ in 0..left_space {
                        result_bytes.push(32);
                    }

                    let mut integer_space = 0;

                    if max_integer_length[&current_column] > current_integer_length {
                        for _ in 0..max_integer_length[&current_column] - current_integer_length {
                            result_bytes.push(32);
                            integer_space += 1;
                        }
                    }

                    for current_byte in start_byte..end_byte {
                        result_bytes.push(byte_array[current_byte as usize]);
                    }

                    let right_space =
                        max_column_width[&current_column] - current_length as usize - integer_space as usize - left_space;

                    if max_column_width[&current_column] > current_length as usize {
                        for _ in 0..right_space {
                            result_bytes.push(32);
                        }
                    }

                    result_bytes.push(32);
                    result_bytes.push(32);
                }
            } else {
                current_length = end_byte as i32 - start_byte as i32;

                for current_byte in start_byte..end_byte {
                    result_bytes.push(byte_array[current_byte as usize]);
                }

                if max_column_width[&current_column] > current_length as usize {
                    for _ in 0..max_column_width[&current_column] - current_length as usize {
                        result_bytes.push(32);
                    }
                }

                result_bytes.push(32);
                result_bytes.push(32);
            }
        }

        result_bytes.push(13);
        result_bytes.push(10);

        n += max_col as u32 + 1;
        current_row += 1;
        if current_row >= total_row as i32 {
            break;
        }
    }    

    println!("{}", String::from_utf8_lossy(&result_bytes));
}

fn max_column_width(byte_array: &Vec<u8>, csv_meta: CsvMeta) -> (bool, HashMap<i32, i32>) {
    
    let cell_address = cell_address(byte_array, csv_meta.clone());
    let is_zero_row = byte_array.is_empty();   
    let extra_line_br_char = if csv_meta.is_line_br_13_exist { 1 } else { 0 };   
    let max_col = csv_meta.total_column + extra_line_br_char - 1;
    let max_col2 = max_col as usize;
    let cell = cell_address.len() - (max_col2 + 1);
    let total_column = csv_meta.total_column;
    let mut n = 0u32;
    let mut current_row = 0i32;    
    let mut temp_bytes: Vec<u8> = Vec::new();    

    let read_csv_delimiter = csv_meta.delimiter;
    let mut max_text_length = HashMap::new();
    let mut max_integer_length = HashMap::new();
    let mut max_decimal_length = HashMap::new();
    let mut max_column_width = HashMap::new();
    let mut is_column_real_number = HashMap::new();    

    for current_column in 0..total_column {
        max_text_length.insert(current_column, 1);
        max_integer_length.insert(current_column, 1);
        max_decimal_length.insert(current_column, 1);
        max_column_width.insert(current_column, csv_meta.column_name[current_column as usize].len() as i32);
        is_column_real_number.insert(current_column, true);
    }

    if !is_zero_row {
        while n < cell as u32 {
            for current_column in 0..total_column {
                if current_column > 0 {
                    temp_bytes.push(read_csv_delimiter);
                }

                let current_cell =
                    (max_col + 1) as i32 * current_row + current_column as i32;
                let start_byte = cell_address[current_cell as usize];
                let end_byte = cell_address[(current_cell + 1) as usize] - 1;
                let (start_byte, end_byte) =
                    skip_white_space(byte_array, start_byte, end_byte);

                let mut is_real_number = true;
                let mut _is_float = false;
                let mut is_negative_sign_exist = false;
                let mut is_open_bracket = false;
                let mut is_close_bracket = false;
                let mut integer_length = 0;
                let mut decimal_length = 0;
                let mut is_dot_exist = false;
                let mut _text_length = 0;

                for current_byte in start_byte..end_byte {
                    match byte_array[current_byte as usize] {
                        b'.' => {
                            if _is_float {
                                // exist more than one time
                                _is_float = false;
                                is_real_number = false;
                            }
                            _is_float = true;
                            if current_byte == start_byte || current_byte == end_byte - 1 {
                                is_real_number = false;
                            }
                        }
                        b'-' => {
                            if is_negative_sign_exist {
                                is_negative_sign_exist = false;
                                is_real_number = false;
                            }
                            if current_byte == start_byte {
                                is_negative_sign_exist = true;
                            } else {
                                is_real_number = false;
                            }
                        }
                        b'(' => {
                            if current_byte == start_byte {
                                is_open_bracket = true;
                            } else {
                                is_real_number = false;
                            }
                        }
                        b')' => {
                            if current_byte == end_byte - 1 {
                                is_close_bracket = true;
                            } else {
                                is_real_number = false;
                            }
                        }
                        _ => {
                            if byte_array[current_byte as usize] < b'0'
                                || byte_array[current_byte as usize] > b'9'
                            {
                                is_real_number = false;
                            }
                        }
                    }
                }
                
                if is_negative_sign_exist {
                    is_real_number = true;
                }

                if is_open_bracket && is_close_bracket {
                    is_real_number = true;
                }

                if is_real_number {
                    for current_byte in start_byte..end_byte {
                        if byte_array[current_byte as usize] == b'.' {
                            is_dot_exist = true;
                        } else if is_dot_exist {
                            decimal_length += 1;
                        } else {
                            integer_length += 1;
                        }
                    }

                    if integer_length > max_integer_length[&current_column] {
                        max_integer_length.insert(current_column, integer_length);
                    }

                    if decimal_length > max_decimal_length[&current_column] {
                        max_decimal_length.insert(current_column, decimal_length);
                    }
                }

                *is_column_real_number.get_mut(&current_column).unwrap() =
                    *is_column_real_number.get(&current_column).unwrap() && is_real_number;

                _text_length = (end_byte - start_byte) as i32;
               
                if _text_length > max_text_length[&current_column] {
                    max_text_length.insert(current_column, _text_length);
                }

                temp_bytes.extend_from_slice(&byte_array[start_byte as usize..end_byte as usize]);
            }
             

            n += (max_col + 1) as u32;
            current_row += 1;
            if current_row > 10 {
                break;
            }
            temp_bytes.clear();
        }
    }

    for current_column in 0..total_column {
    let current_length = max_integer_length[&current_column] + max_decimal_length[&current_column] + 1;

    if is_column_real_number[&current_column] {
        if max_column_width[&current_column] < current_length {
            max_column_width.insert(current_column, current_length);
        }
    } else {
        if max_column_width[&current_column] < max_text_length[&current_column] {
            max_column_width.insert(current_column, max_text_length[&current_column]);
        }
    }
}


    (is_zero_row, max_column_width)
}

#[pyfunction]
fn view_csv(byte_array: Vec<u8>, csv_meta: CsvMeta) {
    let (is_zero_row, max_column_width) = max_column_width(&byte_array, csv_meta.clone());

    if !is_zero_row {
        let mut total_width = 0;
        let mut current_width = 0;

        for i in 0..max_column_width.len() {
            total_width += max_column_width[&(i as i32)];
        }

        let mut total_row = 20;
        let mut table_count = 1;

        println!();

        let mut current_column = 0;
        let mut start_column = 0;

        if total_width > 150 {
            total_row = 8;

            while current_column < max_column_width.len() {
                current_width += max_column_width[&(current_column as i32)];
                if current_width > 100 * table_count {
                    current_view(
                        &byte_array,
                        csv_meta.clone(),
                        start_column,
                        current_column as i32,
                        total_row,
                    );
                    start_column = current_column as i32;
                    table_count += 1;
                }

                current_column += 1;
            }

            current_view(
                &byte_array,
                csv_meta.clone(),
                start_column,
                max_column_width.len() as i32,
                total_row,
            );
        } else if total_width > 100 {
            total_row = 8;

            while current_column < max_column_width.len() {
                current_width += max_column_width[&(current_column as i32)];
                if current_width > total_width / 2 {
                    current_view(
                        &byte_array,
                        csv_meta.clone(),
                        0,
                        current_column as i32,
                        total_row,
                    );
                    start_column = current_column as i32;
                    break;
                }
                current_column += 1;
            }

            current_view(
                &byte_array,
                csv_meta.clone(),
                start_column,
                max_column_width.len() as i32,
                total_row,
            );
        } else {
            current_view(
                &byte_array,
                csv_meta,
                0,
                max_column_width.len() as i32,
                total_row,
            );
        }
    }
}

#[pyfunction]
fn write_csv(byte_array: Vec<u8>, csv_meta: CsvMeta) {
    let mut csv_string = String::new();

    csv_string.push_str(&csv_meta.column_name[0]);

    for i in 1..csv_meta.column_name.len() {
        csv_string.push_str(",");
        csv_string.push_str(&csv_meta.column_name[i]);
    }

    csv_string.push_str("\r\n");

    let f = File::create("%Sample.csv").expect("Unable to create file");
    let mut f = BufWriter::new(f);

    f.write_all(csv_string.as_bytes()).expect("Unable to write data");
    f.write_all(&byte_array).expect("Unable to write data");

    println!("A file named %Sample.csv is created from the rows that executed validation.");
    println!();
}

#[pyfunction]
fn get_csv_sample(filepath: &str, mut sample_row: i32) -> PyResult<(Vec<u8>, CsvMeta)> {
    
    let mut csv_meta = CsvMeta {
        total_column: 0,
        validate_row: 0,
        estimate_row: 0,
        is_line_br_13_exist: false,
        is_line_br_10_exist: false,
        column_name: Vec::new(),
        file_size: 0,
        delimiter: 0,
        error_message: String::new(), 
    };       
    
    let mut _is_error: bool = false;
    let mut error_message = String::new();    
    let mut frequency_distribution_by_sample = HashMap::new();  
    let mut csv_vector = Vec::new();
    let mut start_byte = 0;
    let mut sample_byte_count = 0;  
    let mut n = 0;
  
    let mut _delimiter_scenario = HashMap::new(); 

    let file = File::open(filepath);
    let fileinfo = metadata(filepath);     
    let mut file = file.unwrap();  
    let fileinfo = fileinfo.unwrap();

    csv_meta.file_size = fileinfo.len() as i64;

    // Default output number of sample rows 
    if sample_row <= 0 || csv_meta.file_size <= 10000 {
        sample_row = 10;
    }

    if csv_meta.file_size <= 1000 || sample_row <= 2 {
        sample_row = 2;
    }

    // Column Name
   let (_current_row_byte_count, _frequency_distribution, _current_row_byte) = get_current_row_frequency_distribution(&mut file, 0);
   
    _delimiter_scenario = _frequency_distribution.clone();

    // Data Row
    while n <= sample_row as i64 - 1 {
        start_byte += 1;

        let (_current_row_byte_count, _frequency_distribution, _current_row_byte) = get_current_row_frequency_distribution(&mut file, start_byte);
   
        csv_vector.extend(_current_row_byte);
        sample_byte_count += _current_row_byte_count;
        
        frequency_distribution_by_sample.insert(n, _frequency_distribution.clone());       

        let mut temp_delimiter_scenario = HashMap::new();

        for key in _delimiter_scenario.keys() {
            if frequency_distribution_by_sample[&n].contains_key(key) {
                if frequency_distribution_by_sample[&n][key] == _delimiter_scenario[key] {
                    temp_delimiter_scenario.insert(*key, _delimiter_scenario[key]);
                }
            }
        }

        _delimiter_scenario = temp_delimiter_scenario;

        start_byte = csv_meta.file_size * n / sample_row as i64;
        n += 1;
    }

    csv_meta.validate_row = n;

    // Remove line break from current delimiters
    let mut delimiter_exclude_line_br = HashMap::new();

    for key in _delimiter_scenario.keys() {
        if *key != 10 && *key != 13 {
            delimiter_exclude_line_br.insert(*key, _delimiter_scenario[key]);
        } else {
            if *key == 10 {
                csv_meta.is_line_br_10_exist = true;
            } else if *key == 13 {
                csv_meta.is_line_br_13_exist = true;
            }
        }
    }

    // Remove abc123 from current delimiters
    let mut delimiter_exclude_abc123 = HashMap::new();

    if delimiter_exclude_line_br.contains_key(&44) {

        csv_meta.delimiter = 44;
        csv_meta.total_column = delimiter_exclude_line_br[&44] + 1;

    } else if delimiter_exclude_line_br.len() == 1 {

        for key in delimiter_exclude_line_br.keys() {

            if *key <= 47
                || (*key >= 58 && *key <= 64)
                || (*key >= 91 && *key <= 96)
                || *key >= 123
            {
                csv_meta.delimiter = *key;
                csv_meta.total_column = delimiter_exclude_line_br[key] + 1;
            }
        }

    // If more than one delimiters, this may be recognized as error
    } else if delimiter_exclude_line_br.len() > 1 {

        for key in delimiter_exclude_line_br.keys() {

            if *key <= 47
                || (*key >= 58 && *key <= 64)
                || (*key >= 91 && *key <= 96)
                || *key >= 123
            {
                delimiter_exclude_abc123.insert(*key, delimiter_exclude_line_br[key]);
            }
        }

        if delimiter_exclude_abc123.len() == 1 {

            for key in delimiter_exclude_abc123.keys() {
                csv_meta.delimiter = *key;
                csv_meta.total_column = delimiter_exclude_line_br[key] + 1;
            }
        } else if delimiter_exclude_abc123.len() > 1 {

            error_message.push_str("** More than one possible delimiter ** \n");
            for key in delimiter_exclude_abc123.keys() {
                error_message.push_str(&format!(
                    "   ASCII{} ({})\n",
                    key,
                    str::from_utf8(&[*key]).unwrap()
                ));
            }           
        }
    }

    // Record error messages
   if _is_error == false  {

         csv_meta.column_name = get_column_name(&mut file, csv_meta.delimiter);
         csv_meta.estimate_row =
             csv_meta.file_size as i64 / sample_byte_count as i64 * sample_row as i64;

         if delimiter_exclude_line_br.is_empty() {            
             error_message.push_str("** Fail to find delimiter ** \n");

         } else {
             if csv_meta.total_column == 0 {
                 error_message.push_str("** Fail to count number of column ** \n");
             }

             if csv_meta.estimate_row == 0 {
                 error_message.push_str("** Fail to estimate number of row ** \n");
             }

             if csv_meta.column_name.is_empty() {
                 error_message.push_str("** Fail to find any column name ** \n");
             }

             if csv_meta.column_name.len() != csv_meta.total_column as usize {
                 error_message.push_str("** Number of column name is ");
                 error_message.push_str(&csv_meta.column_name.len().to_string());
                 error_message.push_str(", but number of column is ");
                 error_message.push_str(&csv_meta.total_column.to_string());
                 error_message.push_str(" ** \n");
             }
         }
         csv_meta.error_message = error_message
     }    

     Ok((csv_vector, csv_meta))    
}

#[pymodule]
fn peakrs(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(get_csv_sample, m)?)?;
    m.add_function(wrap_pyfunction!(view_csv, m)?)?;
    m.add_function(wrap_pyfunction!(write_csv, m)?)?;    
    m.add_class::<CsvMeta>().unwrap();
    Ok(())
}