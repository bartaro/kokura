"""Build the locked public CLI and verify it on its native operating system."""
from pathlib import Path
import argparse, hashlib, json, os, platform, re, shutil, struct, subprocess, tempfile, wave

ROOT = Path(__file__).resolve().parents[1]

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def fixture(cgb):
    # Original synthetic test program: initialize audio, then run a stable loop.
    rom = bytearray(32768)
    rom[0x100:0x103] = bytes([0xC3, 0x50, 0x01])
    rom[0x134:0x141] = b'KOKURA NATIVE'
    rom[0x143] = 0x80 if cgb else 0
    code = bytearray([0xF3])
    for reg, val in [(0x26,0x80),(0x24,0x77),(0x25,0xFF),
                     (0x10,0),(0x11,0x80),(0x12,0xF0),(0x13,0x73),(0x14,0x87),
                     (0x16,0x40),(0x17,0xA0),(0x18,0x40),(0x19,0x86),
                     (0x20,0),(0x21,0x60),(0x22,0x37),(0x23,0x80)]:
        code.extend([0x3E,val,0xE0,reg])
    code.extend([0x18,0xFE])
    rom[0x150:0x150+len(code)] = code
    rom[0x14D] = (-sum(rom[0x134:0x14D])-25)&255
    checksum = sum(rom)&65535
    rom[0x14E:0x150] = struct.pack('>H',checksum)
    return rom

def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--platform',required=True)
    ap.add_argument('--target',required=True)
    ap.add_argument('--existing-binary',type=Path,help='Smoke checks only; not a release build receipt')
    args=ap.parse_args()
    dest=ROOT/'dist'/args.platform;dest.mkdir(parents=True,exist_ok=True)
    commands=[]
    def run(label,cmd,env=None):
        proc=subprocess.run(list(map(str,cmd)),cwd=ROOT,env=env,capture_output=True,text=True,encoding='utf-8',errors='replace',timeout=1800)
        output=proc.stdout+proc.stderr
        (dest/(label+'.log')).write_text(output,encoding='utf-8')
        assert proc.returncode==0,(label,proc.returncode,output[-6000:])
        commands.append({'name':label,'arguments':list(map(str,cmd)),'exit_code':proc.returncode})
        return output
    name='kokura-cli.exe' if os.name=='nt' else 'kokura-cli'
    if args.existing_binary:
        binary=args.existing_binary.resolve();count=None
    else:
        log=run('tests',['cargo','test','--locked','--release','-p','kokura-core','-p','kokura-debug','-p','kokura-bridge','-p','kokura-cli','-p','kokura-capi','--lib','--bins','--','--quiet'])
        rows=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',log)
        count=sum(int(r[0]) for r in rows)
        assert count>=304 and all(int(r[1])==0 for r in rows),rows
        env=dict(os.environ)
        if os.name=='nt':env['RUSTFLAGS']=(env.get('RUSTFLAGS','')+' -C target-feature=+crt-static').strip()
        run('release',['cargo','build','--locked','--release','--target',args.target,'-p','kokura-cli','--bin','kokura-cli'],env)
        meta=json.loads(subprocess.check_output(['cargo','metadata','--offline','--locked','--no-deps','--format-version','1'],cwd=ROOT))
        binary=Path(meta['target_directory'])/args.target/'release'/name
    shutil.copy2(binary,dest/name);binary=dest/name
    help_text=run('cli-help',[binary,'--help']);assert '--record-wav' in help_text and '--load-state' in help_text
    checks=[]
    with tempfile.TemporaryDirectory(prefix='kokura-native-') as tmp:
        tmp=Path(tmp)
        for mode in ['dmg','cgb']:
            rom=tmp/(mode+'.gb');rom.write_bytes(fixture(mode=='cgb'))
            wav=tmp/(mode+'.wav');state=tmp/(mode+'.state');report=tmp/(mode+'.json')
            run(mode+'-audio',[binary,rom,'--hardware',mode,'--run-frames','60','--record-wav',wav,'--save-state',state,'--dump-report',report,'--report-minimal','summary'])
            with wave.open(str(wav)) as w:
                channels,width,rate,frames=w.getnchannels(),w.getsampwidth(),w.getframerate(),w.getnframes()
                data=w.readframes(frames)
            assert (channels,width,rate)==(2,2,65536) and 64000<frames<68000
            samples=struct.unpack('<'+'h'*(len(data)//2),data);peak=max(abs(s) for s in samples)
            assert 0<peak<32767 and state.stat().st_size>0
            assert json.loads(report.read_text(encoding='utf-8'))
            checks.append({'case':mode+'-audio','passed':True,'pcm_frames':frames,'sample_rate':rate,'peak':peak,'pcm_sha256':hashlib.sha256(data).hexdigest()})
            resumed=tmp/(mode+'-resumed.wav')
            run(mode+'-resume',[binary,rom,'--hardware',mode,'--load-state',state,'--run-frames','30','--record-wav',resumed])
            with wave.open(str(resumed)) as w:assert w.getframerate()==65536 and 32000<w.getnframes()<34000
            checks.append({'case':mode+'-state-resume','passed':True})
    deps=''
    if platform.system()=='Linux':deps=run('dependencies',['ldd',binary])
    if platform.system()=='Darwin':deps=run('dependencies',['otool','-L',binary])
    files=subprocess.check_output(['git','ls-files','Cargo.toml','Cargo.lock','crates'],cwd=ROOT,text=True).splitlines()
    sources=[{'path':p,'sha256':digest(ROOT/p)} for p in files if Path(p).suffix in ['.rs','.toml','.lock']]
    receipt={'passed':True,'release_build':not bool(args.existing_binary),'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'source_files':sources,'platform':args.platform,'system':platform.system(),'machine':platform.machine(),'target':args.target,'toolchain':subprocess.check_output(['rustc','-Vv'],text=True),'macos_deployment_target':os.environ.get('MACOSX_DEPLOYMENT_TARGET') or None,'test_count':count,'commands':commands,'native_smoke_checks':checks,'binary':{'file':name,'bytes':binary.stat().st_size,'sha256':digest(binary)},'executable_dependencies':deps,'cargo_lock_sha256':digest(ROOT/'Cargo.lock'),'physical_hardware_tested':False}
    (dest/'verification.json').write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({k:receipt[k] for k in ['passed','release_build','source_commit','platform','test_count','binary']}))

if __name__=='__main__':main()
