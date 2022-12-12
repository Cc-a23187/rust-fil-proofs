use std::env::join_paths;
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use bellperson::{Circuit, groth16, Index, SynthesisError, Variable};
use bellperson::gpu::GpuName;
use bellperson::groth16::{ParameterSource, Proof};
use blstrs::{Bls12, Scalar};
use ff::PrimeField;
use itertools::Itertools;
use proptest::collection::vec;
use rand::rngs::OsRng;
use rayon::prelude::*;
use std::process::Command;

#[allow(clippy::type_complexity)]
fn synthesize_circuits_batch<Scalar, C>(
    circuits: Vec<C>,
) -> Result<
    (
        std::vec::Vec<ProvingAssignment<Scalar>>,
        std::vec::Vec<std::sync::Arc<std::vec::Vec<<Scalar as PrimeField>::Repr>>>,
        std::vec::Vec<std::sync::Arc<std::vec::Vec<<Scalar as PrimeField>::Repr>>>,
    ),
    SynthesisError,
>
    where
        Scalar: PrimeField,
        C: Circuit<Scalar> + Send,
{
    let mut provers = circuits
        .into_par_iter()
        .map(|circuit| -> Result<_, SynthesisError> {
            let mut prover = ProvingAssignment::new();

            prover.alloc_input(|| "", || Ok(Scalar::one()))?;

            circuit.synthesize(&mut prover)?;

            for i in 0..prover.input_assignment.len() {
                prover.enforce(|| "", |lc| lc + Variable(Index::Input(i)), |lc| lc, |lc| lc);
            }

            Ok(prover)
        })
        .collect::<Result<Vec<_>, _>>()?;


    // Start fft/multiexp prover timer

    let input_assignments = provers
        .par_iter_mut()
        .map(|prover| {
            let input_assignment = std::mem::take(&mut prover.input_assignment);
            Arc::new(
                input_assignment
                    .into_iter()
                    .map(|s| s.to_repr())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    let aux_assignments = provers
        .par_iter_mut()
        .map(|prover| {
            let aux_assignment = std::mem::take(&mut prover.aux_assignment);
            Arc::new(
                aux_assignment
                    .into_iter()
                    .map(|s| s.to_repr())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    Ok((provers, input_assignments, aux_assignments))
}

//拿到pk文件目录
fn get_groth_params_to_prover_key(groth_params: &groth16::MappedParameters<Bls12>) -> &Path {
    return groth_params.param_file_path.as_path()
}
//写入Assignment到文件并拿到Assignment文件目录
fn get_pub_params_to_assignment(circuits:Vec<C>) -> &Path
    where
        Scalar: PrimeField,
        C: Circuit<Scalar> + Send,{
    // fixme write 2 collection to file
    // &primary
    // &auxiliary
    let p = Path::new("/mnt/lotus/zhangzhichaoHome/tmp");
    let ( _, input_assignments, aux_assignments) =
        synthesize_circuits_batch(circuits)?;
    let mut file = File::create(p)?;
    // Early return on error
    file.write_all(format!("name: {}\n",
                           &input_assignments.len() + &aux_assignments.len()).as_bytes())?;
    file.write_all(format!("name: {}\n", &input_assignments).as_bytes())?;
    file.write_all(format!("name: {}\n", &aux_assignments).as_bytes())?;
    return  p
}
//执行dizk
fn dizk_execute(pk_file_path: &Path, assignment_path: &Path) -> &Path {
    println!("Check input file path, pk_file_path is {:?}, assignment_path is {:?}", pk_file_path, assignment_path);
    // fixme
    // 1 execute docker exec
    // docker exec -it dizk-1118  /bin/bash -c 'mvn package'
    // docker exec -it dizk-1118  /bin/bash -c './scripts/run-prover --partitions 12  --curve "bls12-377"  --output "./proof_10101429.bin" \
    // ./test_data/simple_proving_key_GROTH16_bls12-377.bin ./test_data/simple_assignment_bls12-377.bin > ./test_data/profile_bls12-377_12_1.txt'
    // 2 cp proof file to tmp?
    let out_put = Path::new("/mnt/lotus/zhangzhichaoHome/tmp/proof_10101429.bin");
    let dizk_sh = "\'./scripts/run-prover --partitions 12  --curve \"bls12-377\"  --output \"./proof_10101429.bin\" ./test_data/simple_proving_key_GROTH16_bls12-377.bin ./test_data/simple_assignment_bls12-377.bin > ./test_data/profile_bls12-377_12_1.txt\'";
    let output = Command::new("docker").
        arg("exec").
        arg("-it").
        arg("dizk-1118").
        arg("/bin/bash").
        arg("-c").
        arg(dizk_sh).
        output().expect("命令执行异常错误提示");
    let ls_la_list = String::from_utf8(output.stdout);
    println!(ls_la_list);
    return  out_put
}

//读取dizk prover proof文件中的A B C，转换Vec<Proof<E>
fn read_dizk_proof(dizk_proof_path: &Path) -> Vec<Proof<E>> {
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
        C: Circuit<E::Fr> + Send,
        E::Fr: GpuName,
        E::G1Affine: GpuName,
        E::G2Affine: GpuName,
{
    println!("Execute in dizk");
    pk_file_path = get_groth_params_to_prover_key(params);
    assignment_path = get_pub_params_to_assignment(circuits);
    dizk_proof_path = dizk_execute(pk_file_path, assignment_path);
    return read_dizk_proof(dizk_proof_path)
}

