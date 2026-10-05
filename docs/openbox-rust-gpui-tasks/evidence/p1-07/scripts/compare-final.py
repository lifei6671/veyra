from PIL import Image,ImageCms
from pathlib import Path
import io,json,hashlib
p=Path('docs/openbox-rust-gpui-tasks/evidence/p1-07/final-20261005');binary=json.loads((p/'build-identity.json').read_text())['binary_sha256'];records=[]
def srgb(file):
 im=Image.open(file)
 if im.info.get('icc_profile'):im=ImageCms.profileToProfile(im,ImageCms.ImageCmsProfile(io.BytesIO(im.info['icc_profile'])),ImageCms.createProfile('sRGB'),outputMode='RGB')
 return im.convert('RGB')
for g in sorted((p/'gpui').glob('*.jpg')):
 im=srgb(g);assert im.size==(2560,1504),(g,im.size)
 im=im.crop((0,64,2560,1504)).resize((1280,720),Image.Resampling.LANCZOS)
 normalized=p/'comparison'/(g.stem+'-gpui.png');im.save(normalized)
 alias={'light-shell-collapsed':'light-collapsed','dark-shell-collapsed':'dark-collapsed','light-disabled':'light-panel','dark-disabled':'dark-panel'}
 r=p/'react'/(alias.get(g.stem,g.stem)+'.png')
 record={'state':g.stem,'gpui':str(g.relative_to(p)),'binary_sha256':binary,'logical_content':[1280,720],'crop_physical':[0,64,2560,1504],'icc':'embedded profile → sRGB before crop/resize','sha256':{'gpui':hashlib.sha256(g.read_bytes()).hexdigest()},'verdict':'REGION_REVIEW; see visual-matrix.md; no global similarity score'}
 if r.exists():
  record['react']=str(r.relative_to(p));record['sha256']['react']=hashlib.sha256(r.read_bytes()).hexdigest();Image.blend(srgb(r),im,.5).save(p/'comparison'/(g.stem+'-overlay.png'))
 records.append(record)
(p/'comparison'/'manifest.json').write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n')
