use std::fmt;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::sync::Arc;
use bellperson::{Circuit, ConstraintSystem, groth16, Index, LinearCombination, SynthesisError, Variable};
use bellperson::groth16::{ParameterSource, Proof, synthesize_circuits_batch};
use blstrs::{Bls12, Scalar};
use ff::PrimeField;
use rayon::prelude::*;
use std::process::Command;
use bellperson::multiexp::DensityTracker;
use pairing::MultiMillerLoop;



//拿到pk文件目录
fn get_groth_params_to_prover_key(groth_params: &groth16::MappedParameters<Bls12>) -> Result<&Path, str> {
    Ok(groth_params.param_file_path.as_path())
}
//写入Assignment到文件并拿到Assignment文件目录
fn get_pub_params_to_assignment<E, C, P: ParameterSource<E>>(circuits:Vec<C>) -> Result<&Path, str>
    where
        E: MultiMillerLoop,
        Scalar: PrimeField,
        C: Circuit<Scalar> + Send,{
    // fixme write 2 collection to file
    let p = Path::new("/mnt/lotus/zhangzhichaoHome/tmp");
    let ( _, _, input_assignments, aux_assignments) =
        synthesize_circuits_batch(circuits)?;
    let mut file = File::create(p)?;
    // Early return on error
    file.write_all(format!("name: {}\n",
                           &input_assignments.len() + &aux_assignments.len()).as_bytes())?;
    file.write_all(format!("name: {}\n", &input_assignments).as_bytes())?;
    file.write_all(format!("name: {}\n", &aux_assignments).as_bytes())?;
    Ok(p)
}
//执行dizk
fn dizk_execute(pk_file_path: &Path, assignment_path: &Path, out_put: &Path){
    println!("Check input file path, pk_file_path is {:?}, assignment_path is {:?}", pk_file_path, assignment_path);
    // fixme
    // 1 execute docker exec
    // docker exec -it dizk-1118  /bin/bash -c 'mvn package'
    // docker exec -it dizk-1118  /bin/bash -c './scripts/run-prover --partitions 12  --curve "bls12-377"  --output "./proof_10101429.bin" \
    // ./test_data/simple_proving_key_GROTH16_bls12-377.bin ./test_data/simple_assignment_bls12-377.bin > ./test_data/profile_bls12-377_12_1.txt'
    // 2 cp proof file to tmp?
    let out_put = Path::new("/mnt/lotus/zhangzhichaoHome/tmp/proof_10101429.bin");
    println!("pk_file_path {:?}, assignment_path {:?}", pk_file_path, assignment_path);
    let dizk_sh = "\'./scripts/run-prover --partitions 12  --curve \"bls12-377\" \
     --output \"./proof_10101429.bin\" ./test_data/simple_proving_key_GROTH16_bls12-377.bin \
     ./test_data/simple_assignment_bls12-377.bin > ./test_data/profile_bls12-377_12_1.txt\'";
    let output = Command::new("docker").
        arg("exec").
        arg("-it").
        arg("dizk-1118").
        arg("/bin/bash").
        arg("-c").
        arg(dizk_sh).
        output().expect("命令执行异常错误提示");
    let ls_la_list = String::from_utf8(output.stdout);
    println!("{:?}", ls_la_list);
}

//读取dizk prover proof文件中的A B C，转换Vec<Proof<E>
fn read_dizk_proof<E, C, P: ParameterSource<E>>(dizk_proof_path: &Path) -> Vec<Proof<E>>
where
    E: MultiMillerLoop,
    Scalar: PrimeField,
    C: Circuit<Scalar> + Send,
{
    let f = File::open(dizk_proof_path)?;
    let mut reader = BufReader::new(f);
    let mut v = Vec::new();
    let dizk_proof = Proof::read(reader)?;
    v.push(dizk_proof);
    return v;
    //fixme:可能需要修改成以下形式
    // Ok(Proof {
    // a: g_a.to_affine(),
    // b: g_b.to_affine(),
    // c: g_c.to_affine(),
    // })
}

//与rust-fil-proofs交互接口
pub fn create_dizk_proof_batch<E, C, P: ParameterSource<E>>(
    circuits: Vec<C>,
    params: P,
) -> Vec<Proof<E>>
where
    E: MultiMillerLoop,
    Scalar: PrimeField,
    C: Circuit<Scalar> + Send,
{
    println!("Execute in dizk");
    let pk_file_path = get_groth_params_to_prover_key(params)?;
    let assignment_path = get_pub_params_to_assignment(circuits)?;
    let out_put_path = Path::new("/mnt/lotus/zhangzhichaoHome/tmp/proof_10101429.bin");
    dizk_execute(pk_file_path, assignment_path, out_put_path);
    return read_dizk_proof(out_put_path)
}

