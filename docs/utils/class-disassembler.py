import struct, sys
OPS = "nop aconst_null iconst_m1 iconst_0 iconst_1 iconst_2 iconst_3 iconst_4 iconst_5 lconst_0 lconst_1 fconst_0 fconst_1 fconst_2 dconst_0 dconst_1 bipush sipush ldc ldc_w ldc2_w iload lload fload dload aload iload_0 iload_1 iload_2 iload_3 lload_0 lload_1 lload_2 lload_3 fload_0 fload_1 fload_2 fload_3 dload_0 dload_1 dload_2 dload_3 aload_0 aload_1 aload_2 aload_3 iaload laload faload daload aaload baload caload saload istore lstore fstore dstore astore istore_0 istore_1 istore_2 istore_3 lstore_0 lstore_1 lstore_2 lstore_3 fstore_0 fstore_1 fstore_2 fstore_3 dstore_0 dstore_1 dstore_2 dstore_3 astore_0 astore_1 astore_2 astore_3 iastore lastore fastore dastore aastore bastore castore sastore pop pop2 dup dup_x1 dup_x2 dup2 dup2_x1 dup2_x2 swap iadd ladd fadd dadd isub lsub fsub dsub imul lmul fmul dmul idiv ldiv fdiv ddiv irem lrem frem drem ineg lneg fneg dneg ishl lshl ishr lshr iushr lushr iand land ior lor ixor lxor iinc i2l i2f i2d l2i l2f l2d f2i f2l f2d d2i d2l d2f i2b i2c i2s lcmp fcmpl fcmpg dcmpl dcmpg ifeq ifne iflt ifge ifgt ifle if_icmpeq if_icmpne if_icmplt if_icmpge if_icmpgt if_icmple if_acmpeq if_acmpne goto jsr ret tableswitch lookupswitch ireturn lreturn freturn dreturn areturn return getstatic putstatic getfield putfield invokevirtual invokespecial invokestatic invokeinterface invokedynamic new newarray anewarray arraylength athrow checkcast instanceof monitorenter monitorexit wide multianewarray ifnull ifnonnull goto_w jsr_w".split()
LEN1 = {"bipush","ldc","iload","lload","fload","dload","aload","istore","lstore","fstore","dstore","astore","ret","newarray"}
LEN2 = {"sipush","ldc_w","ldc2_w","iinc","getstatic","putstatic","getfield","putfield","invokevirtual","invokespecial","invokestatic","new","anewarray","checkcast","instanceof","ifeq","ifne","iflt","ifge","ifgt","ifle","if_icmpeq","if_icmpne","if_icmplt","if_icmpge","if_icmpgt","if_icmple","if_acmpeq","if_acmpne","goto","jsr","ifnull","ifnonnull"}
def parse(path):
    d=open(path,'rb').read(); p=8
    n=struct.unpack('>H',d[p:p+2])[0]; p+=2
    cp=[None]*n; i=1
    while i<n:
        t=d[p]; p+=1
        if t==1:
            l=struct.unpack('>H',d[p:p+2])[0]; p+=2; cp[i]=('utf8',d[p:p+l].decode('utf8','replace')); p+=l
        elif t in (3,4): cp[i]=('num',struct.unpack('>i',d[p:p+4])[0]); p+=4
        elif t in (5,6): cp[i]=('num8',d[p:p+8]); p+=8; i+=1
        elif t in (7,8,16,19,20): cp[i]=({7:'class',8:'str',16:'mtype',19:'mod',20:'pkg'}[t],struct.unpack('>H',d[p:p+2])[0]); p+=2
        elif t in (9,10,11,12,17,18): cp[i]=({9:'field',10:'meth',11:'imeth',12:'nat',17:'dyn',18:'indy'}[t],struct.unpack('>HH',d[p:p+4])); p+=4
        elif t==15: cp[i]=('mh',d[p:p+3]); p+=3
        i+=1
    def u(i): return cp[i][1]
    def res(i):
        e=cp[i]
        if e is None: return '?'
        k=e[0]
        if k=='utf8': return e[1]
        if k in('class','str'): return res(e[1])
        if k=='nat': return res(e[1][0])+':'+res(e[1][1])
        if k in('field','meth','imeth'): return res(e[1][0])+'.'+res(e[1][1])
        if k=='num': return str(e[1])
        return k
    acc,this,sup,ni=struct.unpack('>HHHH',d[p:p+8]); p+=8+2*ni
    def attrs(p):
        na=struct.unpack('>H',d[p:p+2])[0]; p+=2; out=[]
        for _ in range(na):
            nm,ln=struct.unpack('>HI',d[p:p+6]); p+=6; out.append((u(nm),d[p:p+ln])); p+=ln
        return out,p
    nf=struct.unpack('>H',d[p:p+2])[0]; p+=2
    for _ in range(nf):
        p+=6; _,p=attrs(p)
    nm=struct.unpack('>H',d[p:p+2])[0]; p+=2
    methods=[]
    for _ in range(nm):
        a,n1,dsc=struct.unpack('>HHH',d[p:p+6]); p+=6; at,p=attrs(p)
        methods.append((u(n1),u(dsc),at))
    return cp,res,methods
def dis(code,res):
    p=0; out=[]
    while p<len(code):
        op=code[p]; nm=OPS[op]; off=p; p+=1; arg=''
        if nm in LEN1:
            v=code[p]; p+=1
            arg=res(v) if nm=='ldc' else str(v)
        elif nm in LEN2:
            v=struct.unpack('>H',code[p:p+2])[0]; p+=2
            if nm.startswith('if') or nm in('goto','jsr'):
                v=struct.unpack('>h',code[p-2:p])[0]; arg=str(off+v)
            elif nm=='iinc': arg=str(code[p-2])+','+str(code[p-1]) 
            else:
                try: arg=res(v)
                except Exception: arg='#%d'%v
        elif nm in('invokeinterface','invokedynamic'):
            v=struct.unpack('>H',code[p:p+2])[0]; p+=4; arg=res(v)
        elif nm=='ldc_w': pass
        elif nm=='multianewarray': p+=3
        elif nm=='wide':
            p+=3 if code[p]!=0x84 else 5
        elif nm in('goto_w','jsr_w'): p+=4
        elif nm=='tableswitch':
            p=(p+3)&~3; lo,hi=struct.unpack('>ii',code[p+4:p+12]); p+=12+4*(hi-lo+1)
        elif nm=='lookupswitch':
            p=(p+3)&~3; npairs=struct.unpack('>i',code[p+4:p+8])[0]; p+=8+8*npairs
        out.append(f"{off:4d}: {nm} {arg}")
    return out
if __name__=='__main__':
    cp,res,methods=parse(sys.argv[1])
    want=sys.argv[2] if len(sys.argv)>2 else None
    for name,dsc,at in methods:
        if want and want!=name: continue
        for an,ab in at:
            if an=='Code':
                ml,cl=struct.unpack('>HH',ab[:4]) if False else (None,None)
                msl,mlo,clen=struct.unpack('>HHI',ab[:8]); code=ab[8:8+clen]
                print(f"== {name}{dsc} len={clen}")
                for l in dis(code,res): print(l)
