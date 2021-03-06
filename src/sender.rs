use bytes::BufMut;
use bytes::BytesMut;
use cita_cloud_proto::blockchain::{Transaction, UnverifiedTransaction, Witness};
use cita_cloud_proto::common::Empty;
use cita_cloud_proto::controller::{
    raw_transaction::Tx, rpc_service_client::RpcServiceClient, Flag, RawTransaction,
};
use cita_cloud_proto::kms::{
    kms_service_client::KmsServiceClient, GenerateKeyPairRequest, HashDataRequest,
    SignMessageRequest,
};
use prost::Message;
use tonic::transport::channel::Channel;
use tonic::Request;

pub struct Sender {
    cc_name: String,
    chain_id: Vec<u8>,
    start_block_number: u64,
    key_id: u64,
    address: Vec<u8>,

    kms_client: KmsServiceClient<Channel>,
    rpc_client: RpcServiceClient<Channel>,
}

impl Sender {
    pub async fn new(cc_name: &str, kms_addr: &str, controller_addr: &str) -> Self {
        let cc_name = cc_name.to_string();
        let mut kms_client = {
            let kms_addr = format!("http://{}", kms_addr);
            KmsServiceClient::connect(kms_addr).await.unwrap()
        };
        let mut rpc_client = {
            let controller_addr = format!("http://{}", controller_addr);
            RpcServiceClient::connect(controller_addr).await.unwrap()
        };

        // generate key pair for signing tx
        let (key_id, address) = {
            let request = Request::new(GenerateKeyPairRequest {
                crypt_type: 1,
                description: "test".to_owned(),
            });
            let resp = kms_client
                .generate_key_pair(request)
                .await
                .unwrap()
                .into_inner();
            (resp.key_id, resp.address)
        };

        // get system config
        let sys_config = {
            let request = Request::new(Empty {});
            rpc_client
                .get_system_config(request)
                .await
                .unwrap()
                .into_inner()
        };
        let chain_id = sys_config.chain_id;

        // get start block number
        let start_block_number = {
            let request = Request::new(Flag { flag: false });
            rpc_client
                .get_block_number(request)
                .await
                .unwrap()
                .into_inner()
                .block_number
        };

        Self {
            cc_name,
            chain_id,
            start_block_number,
            key_id,
            address,
            kms_client,
            rpc_client,
        }
    }

    pub async fn send(&mut self, proposal: &[u8]) {
        let mut data = BytesMut::new();
        data.put_u64(self.cc_name.len() as u64);
        data.put_slice(self.cc_name.as_bytes());
        data.put_slice(proposal);

        let tx = build_tx(
            data.to_vec(),
            self.start_block_number,
            self.chain_id.clone(),
        );

        // calc tx hash
        let tx_hash = {
            let tx_bytes = {
                let mut buf = Vec::new();
                tx.encode(&mut buf).unwrap();
                buf
            };
            let request = HashDataRequest {
                key_id: self.key_id,
                data: tx_bytes,
            };
            self.kms_client
                .hash_data(request)
                .await
                .unwrap()
                .into_inner()
                .hash
        };

        // sign tx hash
        let signature = {
            let request = Request::new(SignMessageRequest {
                key_id: self.key_id,
                msg: tx_hash.clone(),
            });
            self.kms_client
                .sign_message(request)
                .await
                .unwrap()
                .into_inner()
                .signature
        };

        // send raw tx
        let raw_tx = {
            let witness = Witness {
                signature,
                sender: self.address.clone(),
            };

            let unverified_tx = UnverifiedTransaction {
                transaction: Some(tx),
                transaction_hash: tx_hash.clone(),
                witness: Some(witness),
            };

            RawTransaction {
                tx: Some(Tx::NormalTx(unverified_tx)),
            }
        };
        let ret_hash = self
            .rpc_client
            .send_raw_transaction(raw_tx)
            .await
            .unwrap()
            .into_inner()
            .hash;
        assert_eq!(ret_hash, tx_hash);
    }
}

fn build_tx(data: Vec<u8>, start_block_number: u64, chain_id: Vec<u8>) -> Transaction {
    Transaction {
        version: 0,
        to: vec![1u8; 21],
        nonce: "test".to_owned(),
        quota: 300_000,
        valid_until_block: start_block_number + 99,
        data,
        value: vec![0u8; 32],
        chain_id,
    }
}
