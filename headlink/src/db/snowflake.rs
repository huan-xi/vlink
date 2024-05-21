use snowdon::{Epoch, Generator, Layout, Snowflake};
use std::sync::Arc;

fn machine_id() -> u64 {
    // This is where you would implement your machine ID
    0
}

// We make our structure hidden, as this is an implementation detail that most
// users of our custom ID won't need
#[derive(Debug)]
#[doc(hidden)]
pub struct MySnowflakeParams;

#[derive(Debug)]
#[doc(hidden)]
pub struct NormalSnowflakeParams;

// 这是因为JS的number类型最大精度为2^53，即9007199254740992
// 可对此53位做以下分配
// （1）时间戳(41位)-工作机器id(6位)-序列号(6位)：最大支持64个workerId, 每毫秒生成64个序列号  系统运行至2092年达到最大值
// （2）时间戳(39位)-工作机器id(4位)-序列号(10位)：最大支持16个workerId, 每毫秒生成1024个序列号
//
// （2）时间戳(42位)-工作机器id(3位)-序列号(8位)：最大支持8个workerId, 每毫秒生成256个序列号

impl Layout for MySnowflakeParams {
    fn construct_snowflake(timestamp: u64, sequence_number: u64) -> u64 {
        assert!(!Self::exceeds_timestamp(timestamp) && !Self::exceeds_sequence_number(sequence_number));
        //二进制输出timestamp
        // println!("{:b}", timestamp);
        // println!("{:b}", sequence_number);
        // 41-14|13-10|0-9
        (timestamp << 14) | (machine_id() << 13) | sequence_number
    }
    fn timestamp(input: u64) -> u64 {
        // 留出14位 给其他
        input >> 14
    }
    fn exceeds_timestamp(input: u64) -> bool {
        input >= (1 << 42)
    }
    fn sequence_number(input: u64) -> u64 {
        input & ((1 << 9) - 1)
    }
    fn exceeds_sequence_number(input: u64) -> bool {
        input >= (1 << 9)
    }
    fn is_valid_snowflake(input: u64) -> bool {
        // Our snowflake format doesn't have any constant parts that we could
        // validate here
        true
    }
}


// 正常64位雪花算法
impl Layout for NormalSnowflakeParams {
    fn construct_snowflake(timestamp: u64, sequence_number: u64) -> u64 {
        assert!(
            !Self::exceeds_timestamp(timestamp)
                && !Self::exceeds_sequence_number(sequence_number)
        );
        (timestamp << 22) | (machine_id() << 12) | sequence_number
    }
    fn timestamp(input: u64) -> u64 {
        input >> 22
    }
    fn exceeds_timestamp(input: u64) -> bool {
        input >= (1 << 42)
    }
    fn sequence_number(input: u64) -> u64 {
        input & ((1 << 12) - 1)
    }
    fn exceeds_sequence_number(input: u64) -> bool {
        input >= (1 << 12)
    }
    fn is_valid_snowflake(input: u64) -> bool {
        // Our snowflake format doesn't have any constant parts that we could
        // validate here
        true
    }
}

impl Epoch for MySnowflakeParams {
    fn millis_since_unix() -> u64 {
        // Our epoch for this example is the first millisecond of 2020
        1577836800000
    }
}

// Define our snowflake and generator types
pub type MySnowflake = Snowflake<MySnowflakeParams, MySnowflakeParams>;

#[derive(Clone)]
pub struct MySnowflakeGenerator {
    inner: Arc<Generator<MySnowflakeParams, MySnowflakeParams>>,
}

impl Default for MySnowflakeGenerator {
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl MySnowflakeGenerator {
    pub fn next_id(&self) -> i64 {
        self.inner.clone().generate().unwrap().into_inner() as i64
        // self.0.generate().unwrap().into_inner()
    }
}

// pub type MySnowflakeGenerator = ;

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use crate::db::snowflake::MySnowflakeGenerator;

    // #[tokio::test]
    // pub async fn test() {
    //     let mut a = MySnowflakeGenerator::default();
    //     let ids = Arc::new(Mutex::new(Vec::new()));
    //     let size = 1000;
    //     for i in 0..1000 {
    //         // println!("{:?}", a.next_id());
    //         let a = a.clone();
    //         let ids = ids.clone();
    //         tokio::spawn(async move {
    //             // let id = a.generate().unwrap();
    //             // ids.push(a.next_id());
    //             let id = a.next_id();
    //             tokio::spawn(async move {
    //                 ids.lock().await.push(id);
    //             });
    //         });
    //     }
    //     // ids.distinct();
    //
    //     tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    //     let unique_vec: Vec<_> = ids.clone().lock().await.clone().into_iter().collect();
    //     assert_eq!(unique_vec.len(), size);
    // }
}