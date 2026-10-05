"""保留原图；按嵌入 ICC 转 sRGB，再统一逻辑尺寸。叠图不是相似度 PASS。"""
from PIL import Image, ImageCms
from pathlib import Path
import io, json, hashlib
p = Path('docs/openbox-rust-gpui-tasks/evidence/p1-07')
def srgb(file):
 im=Image.open(file)
 if im.info.get('icc_profile'):
  im=ImageCms.profileToProfile(im, ImageCms.ImageCmsProfile(io.BytesIO(im.info['icc_profile'])), ImageCms.createProfile('sRGB'), outputMode='RGB')
 return im.convert('RGB')
records=[]
for theme in ['light','dark']:
 pairs=[(state,p/'react'/f'{theme}-{"switch-toast" if state=="switch" else state}.png',p/'gpui'/f'{theme}-{state}.jpg','shell-owned regions only' if state in ['shell','collapsed'] else 'current P1 component geometry; body background/font differences retained') for state in ['shell','collapsed','panel','select','switch','modal','icon-picker','slider']]
 for state in ['tooltip-hover','tooltip-focus','input-focus','icon-button-hover','modal-open','modal-focus-return','switch-focus-off','switch-focus-on','toast-generic','ip-select']:
  gp={'tooltip-focus':'tooltip-keyboard','ip-select':'select'}.get(state,state)
  pairs.append((state,p/'react'/f'{theme}-{state}.png',p/'gpui/interactions'/f'{theme}-{gp}.jpg','component only; same None background, opacity90 blur10 radius16; timeout6500; no global PASS'))
 for state,r,g,scope in pairs:
  if not(r.exists() and g.exists()):continue
  im=srgb(g);ref=srgb(r)
  if im.size!=(2560,1504):records.append({'file':str(g),'status':'OLDER_VIEWPORT','size':im.size});continue
  im=im.crop((0,64,2560,1504)).resize((1280,720),Image.Resampling.LANCZOS)
  im.save(p/'comparison'/f'{theme}-{state}-gpui.png');Image.blend(ref,im,.5).save(p/'comparison'/f'{theme}-{state}-overlay.png')
  records.append({'state':f'{theme}-{state}','react':str(r.relative_to(p)),'gpui':str(g.relative_to(p)),'sha256':{'react':hashlib.sha256(r.read_bytes()).hexdigest(),'gpui':hashlib.sha256(g.read_bytes()).hexdigest()},'color_conversion':'embedded macOS Display ICC → sRGB via ImageCms before crop/resize; React sRGB','content_crop_physical':[0,64,2560,1504],'logical_size':[1280,720],'scope':scope,'status':'REGION_REVIEW; see visual-matrix.md for individual result and retained differences','exclusions':['business body','unavailable Runtime values','native title/menu/file panel','font equivalence not claimed']})
(p/'comparison/manifest.json').write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n')
